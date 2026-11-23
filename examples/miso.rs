//! MISO engine: several input sources, synchronized, one output stream.
//!
//! - NYA1 receiver: 48h of observations, streamed day after day (chained, lazily opened):
//!   only one day is held in memory at a time.
//! - 48h of ephemerides (GPS + Galileo), merged into a single source.
//! - Built-in processor (ephemeris coverage) + application specific processor,
//!   both producing into the same, application defined, output type.
//!
//! ```bash
//! cargo run --release --example miso
//! ```
#[path = "common/mod.rs"]
mod common;

use gnss_qc::{io::rinex::QcRinexObsSource, prelude::*};
use rinex::prelude::Rinex;

/// Application output: built-in results + custom results
#[derive(Debug)]
enum AppOutput {
    Builtin(QcOutput),
    EpochSummary { receivers: usize, signals: usize },
}

impl From<QcOutput> for AppOutput {
    fn from(output: QcOutput) -> Self {
        Self::Builtin(output)
    }
}

/// Custom processor: summarizes each synchronized epoch
struct Summary;

impl QcProcessor<AppOutput> for Summary {
    fn name(&self) -> &str {
        "summary"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<AppOutput>) {
        output.push(
            ctx.epoch,
            AppOutput::EpochSummary {
                receivers: ctx.receivers.len(),
                signals: ctx.receivers.iter().map(|rx| rx.signals.len()).sum(),
            },
        );
    }
}

fn data(file: &str) -> String {
    format!("{}/data/{}", env!("CARGO_MANIFEST_DIR"), file)
}

fn main() {
    let nya1 = QcIndexing::Receiver("TRIMBLE NETR9 #5207K82137".to_string());

    // 48h of observations: two daily files, opened when the engine needs them
    let daily = |file: &'static str, indexing: QcIndexing| {
        QcLazySource::new(
            QcSourceDescriptor::new(indexing.clone()).with_signals(),
            move || {
                let rinex = Rinex::from_gzip_file(data(file))?;
                Ok(Box::new(QcRinexObsSource::new(indexing, rinex)?) as Box<dyn QcSource>)
            },
        )
    };

    let observations = QcChainSource::new(
        QcSourceDescriptor::new(nya1.clone()).with_signals(),
        vec![
            Box::new(daily(
                "CRNX/V3/NYA100NOR_S_20241270000_01D_30S_MO.crx.gz",
                nya1.clone(),
            )),
            Box::new(daily(
                "CRNX/V3/NYA100NOR_S_20241280000_01D_30S_MO.crx.gz",
                nya1.clone(),
            )),
        ],
    );

    // 48h of ephemerides, merged by the products table (custom alias)
    let cfg = QcInputConfig {
        nav_rinex_indexing: Some(QcIndexingConfig::Custom("NYA1-BRDC".to_string())),
        ..Default::default()
    };

    let ephemerides = common::load_products(
        &cfg,
        &[
            "NAV/V3/NYA100NOR_S_20241270000_01D_GN.rnx.gz",
            "NAV/V3/NYA100NOR_S_20241270000_01D_EN.rnx.gz",
            "NAV/V3/NYA100NOR_S_20241280000_01D_GN.rnx.gz",
            "NAV/V3/NYA100NOR_S_20241280000_01D_EN.rnx.gz",
        ],
    );

    let engine = QcEngine::<AppOutput>::builder()
        .with_source(observations)
        .with_products(ephemerides)
        .with_processor(QcEphemerisCoverageProcessor::default())
        .with_processor(Summary)
        .build()
        .unwrap();

    let (mut epochs, mut signals, mut coverage_changes) = (0, 0, 0);

    // blocking consumption through a sink (closure)
    let mut sink = QcFnSink(|item: QcOutputItem<AppOutput>| {
        match item.payload {
            AppOutput::EpochSummary {
                signals: n,
                receivers,
            } => {
                assert_eq!(receivers, 1);
                epochs += 1;
                signals += n;
            }
            AppOutput::Builtin(QcOutput::EphemerisCoverage(cov)) => {
                coverage_changes += 1;
                if cov.source.is_none() {
                    println!("{} - {} lost ephemeris coverage", item.epoch, cov.sv);
                }
            }
            _ => {}
        }
        Ok::<(), ()>(())
    });

    let stats = engine.run(&mut sink).unwrap();

    println!(
        "{} epochs, {} signals, {} coverage changes",
        epochs, signals, coverage_changes
    );
    println!("{:?}", stats);
}
