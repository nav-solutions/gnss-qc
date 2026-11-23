//! MISO core demonstrations on real data (requires the "data" submodule).
//!
//! - single: one OBS, one CRINEX, one NAV file
//! - combined: 48h of the same receiver (merged or chained), 48h of ephemerides
//! - rtk: two different receivers processed simultaneously
//! - realtime: a file replayed through a "socket" (channel) gives identical results
mod combined;
mod realtime;
mod rtk;
mod single;

use std::{
    path::PathBuf,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use rinex::prelude::Rinex;

pub use crate::io::input::cfg::QcIndexingConfig;
pub use crate::prelude::*;

/// Absolute path to a file of the data submodule
pub fn data_path(relative: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join(relative);
    assert!(
        path.exists(),
        "{} does not exist: run git submodule update --init",
        path.display()
    );
    path
}

/// Parses a RINEX file of the data submodule
pub fn load_rinex(relative: &str) -> Rinex {
    let path = data_path(relative);
    if relative.ends_with(".gz") {
        Rinex::from_gzip_file(&path).unwrap()
    } else {
        Rinex::from_file(&path).unwrap()
    }
}

/// Loads files of the data submodule into a [QcInputProducts] table
pub fn load_products(cfg: &QcInputConfig, files: &[&str]) -> QcInputProducts {
    let mut products = QcInputProducts::default();
    for file in files {
        let path = data_path(file);

        #[cfg(feature = "sp3")]
        if file.starts_with("SP3/") {
            let sp3 = sp3::prelude::SP3::from_gzip_file(&path).unwrap();
            products.load_sp3(sp3).unwrap();
            continue;
        }

        let rinex = load_rinex(file);
        products.load_rinex(cfg, &path, rinex).unwrap();
    }
    products
}

/// Summary of one receiver, at one epoch
#[derive(Debug, Clone, PartialEq)]
pub struct ReceiverSummary {
    pub receiver: QcIndexing,
    pub signals: usize,
    pub satellites: Vec<SV>,
}

/// Summary of one processed epoch
#[derive(Debug, Clone, PartialEq)]
pub struct EpochSummary {
    pub epoch: Epoch,
    pub receivers: Vec<ReceiverSummary>,
    pub buffered_ephemerides: usize,
}

/// Coverage statistics, for selected constellations
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CoverageStats {
    /// (epoch, receiver, sv) in sight with a valid ephemeris
    pub covered: usize,
    /// (epoch, receiver, sv) in sight without valid ephemeris
    pub uncovered: usize,
}

impl CoverageStats {
    pub fn ratio(&self) -> f64 {
        self.covered as f64 / (self.covered + self.uncovered) as f64
    }
}

/// Test output: built-in results + test specific results.
/// This is also a demonstration of an application specific output type.
#[derive(Debug, Clone, PartialEq)]
pub enum TestOutput {
    Builtin(QcOutput),
    Epoch(EpochSummary),
    Coverage(CoverageStats),
}

impl From<QcOutput> for TestOutput {
    fn from(output: QcOutput) -> Self {
        Self::Builtin(output)
    }
}

/// Summarizes each epoch, and verifies the core invariants on the fly.
#[derive(Default)]
pub struct EpochRecorder {
    pub last: Option<Epoch>,
    /// Optional assertion: the flag must not be raised before this epoch
    pub not_before: Option<(Arc<AtomicBool>, Epoch)>,
}

impl QcProcessor<TestOutput> for EpochRecorder {
    fn name(&self) -> &str {
        "epoch-recorder"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<TestOutput>) {
        if let Some(last) = self.last {
            assert!(ctx.epoch > last, "{} processed after {}", ctx.epoch, last);
        }
        self.last = Some(ctx.epoch);

        if let Some((flag, not_before)) = &self.not_before {
            if ctx.epoch < *not_before {
                assert!(
                    !flag.load(Ordering::SeqCst),
                    "flag raised too early: {}",
                    ctx.epoch
                );
            }
        }

        let receivers = ctx
            .receivers
            .iter()
            .map(|rx| {
                let mut satellites = rx.signals.iter().map(|sig| sig.sv).collect::<Vec<_>>();
                satellites.sort();
                satellites.dedup();
                ReceiverSummary {
                    receiver: rx.receiver.clone(),
                    signals: rx.signals.len(),
                    satellites,
                }
            })
            .collect();

        output.push(
            ctx.epoch,
            TestOutput::Epoch(EpochSummary {
                epoch: ctx.epoch,
                receivers,
                buffered_ephemerides: ctx.ephemerides.len(),
            }),
        );
    }
}

/// Counts ephemeris coverage of the satellites in sight, emits the statistics at the end.
pub struct CoverageCounter {
    constellations: Vec<Constellation>,
    window: Option<(Epoch, Epoch)>,
    stats: CoverageStats,
}

impl CoverageCounter {
    pub fn new(constellations: &[Constellation]) -> Self {
        Self {
            constellations: constellations.to_vec(),
            window: None,
            stats: Default::default(),
        }
    }

    /// Restricts the statistics to [start, end]
    pub fn within(mut self, start: Epoch, end: Epoch) -> Self {
        self.window = Some((start, end));
        self
    }
}

impl QcProcessor<TestOutput> for CoverageCounter {
    fn name(&self) -> &str {
        "coverage-counter"
    }

    fn process(&mut self, ctx: &QcEpochContext, _: &mut QcOutputQueue<TestOutput>) {
        if let Some((start, end)) = self.window {
            if ctx.epoch < start || ctx.epoch > end {
                return;
            }
        }

        for rx in ctx.receivers {
            let mut satellites = rx
                .signals
                .iter()
                .map(|sig| sig.sv)
                .filter(|sv| self.constellations.contains(&sv.constellation))
                .collect::<Vec<_>>();
            satellites.sort();
            satellites.dedup();

            for sv in satellites {
                if ctx.ephemerides.select(sv, ctx.epoch).is_some() {
                    self.stats.covered += 1;
                } else {
                    self.stats.uncovered += 1;
                }
            }
        }
    }

    fn finalize(&mut self, output: &mut QcOutputQueue<TestOutput>) {
        output.push(Epoch::default(), TestOutput::Coverage(self.stats));
    }
}

/// Results of a test run, streamed through a [QcFnSink]
/// (signals are counted, not stored: 48h of signals is several million items).
#[derive(Debug, Default)]
pub struct RunResults {
    pub epochs: Vec<EpochSummary>,
    pub signals: usize,
    pub signals_per_receiver: std::collections::HashMap<QcIndexing, usize>,
    pub ephemerides: Vec<QcEphemerisOutput>,
    /// One entry per coverage counter, in processors order
    pub coverage: Vec<CoverageStats>,
    pub stats: QcEngineStats,
}

pub fn run(engine: QcEngine<TestOutput>) -> RunResults {
    let mut results = RunResults::default();

    let stats = {
        let mut sink = QcFnSink(|item: QcOutputItem<TestOutput>| {
            match item.payload {
                TestOutput::Epoch(summary) => results.epochs.push(summary),
                TestOutput::Coverage(stats) => results.coverage.push(stats),
                TestOutput::Builtin(QcOutput::Signal(signal)) => {
                    results.signals += 1;
                    *results
                        .signals_per_receiver
                        .entry(signal.receiver)
                        .or_default() += 1;
                }
                TestOutput::Builtin(QcOutput::Ephemeris(eph)) => results.ephemerides.push(eph),
                TestOutput::Builtin(_) => {}
            }
            Ok::<(), ()>(())
        });

        engine.run(&mut sink).unwrap()
    };

    results.stats = stats;
    results
}

/// Ground truth, obtained directly from an Observation [Rinex]
pub struct ObsTruth {
    pub epochs: Vec<(Epoch, usize)>,
    pub signals: usize,
}

impl ObsTruth {
    pub fn new(rinex: &Rinex) -> Self {
        let record = rinex.record.as_obs().expect("not an observation RINEX");
        let epochs = record
            .iter()
            .map(|(k, obs)| (k.epoch, obs.signals.len()))
            .collect::<Vec<_>>();
        let signals = epochs.iter().map(|(_, n)| n).sum();
        Self { epochs, signals }
    }
}

/// Verifies per epoch content of a single receiver run against the ground truth
pub fn assert_matches_truth(results: &RunResults, truth: &ObsTruth, receiver: &QcIndexing) {
    assert_eq!(results.epochs.len(), truth.epochs.len(), "number of epochs");

    for (summary, (epoch, nb_signals)) in results.epochs.iter().zip(truth.epochs.iter()) {
        assert_eq!(summary.epoch, *epoch);
        assert_eq!(summary.receivers.len(), 1, "{}: receivers", epoch);
        assert_eq!(&summary.receivers[0].receiver, receiver);
        assert_eq!(
            summary.receivers[0].signals, *nb_signals,
            "{}: signals",
            epoch
        );
    }

    assert_eq!(results.signals, truth.signals, "total signals");
    assert_eq!(results.stats.late_items, 0);
}
