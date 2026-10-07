//! The MISO (Multiple Inputs, Single Output) processing engine.
//!
//! ```text
//!  OBS (receiver A) ──┐
//!  OBS (receiver B) ──┤                ┌─ processor 1 ─┐
//!  NAV (agency X)  ───┼──► QcEngine ───┼─ processor 2 ─┼──► output stream (pulled by app)
//!  RTCM socket     ───┤   (sync/FIFO)  └─ processor N ─┘
//!  ANTEX (static)  ───┘
//! ```
//!
//! The engine is entirely *pull* based: the application pulls results
//! ([QcEngine::poll_next] or the [Iterator] implementation), which makes the engine
//! pull just enough input data points to produce them.
//!
//! ## Epoch driven processing
//! Signal observations drive the processing: epochs are processed in chronological order,
//! over the union of all signal sources (they do not need to be aligned nor to cover
//! the same time frame). Before an epoch `t` is processed:
//! - every live signal source must have moved past `t` (or ended), so the epoch is complete
//!   for every receiver. This is the only blocking condition in real-time.
//! - every live context source is pulled until it covers `t + lookahead`. In real-time,
//!   a pending context source never blocks processing.
//! - context buffers are pruned (expired data points are dropped).
use std::{collections::HashMap, time::Duration as StdDuration};

use hifitime::{Duration, Epoch};

use rinex::prelude::{
    obs::{ClockObservation, EpochFlag, SignalObservation},
    Rinex,
};

use thiserror::Error;

#[cfg(feature = "log")]
use log::{debug, warn};

use crate::{
    io::{
        input::indexing::QcIndexing,
        output::{QcOutput, QcOutputItem, QcOutputQueue, QcSink},
        source::{
            QcInputItem, QcPayload, QcPoll, QcSignal, QcSource, QcSourceDescriptor, QcSourceError,
        },
    },
    sync::QcEphemerisStore,
};

/// Errors raised by the engine.
#[derive(Debug, Error)]
pub enum QcEngineError {
    /// Processing is driven by signal observations:
    /// at least one source must provide signals.
    #[error("no source provides signal observations")]
    NoSignalSource,

    /// Two signal sources share the same identity:
    /// they must be merged (or indexed differently) prior processing.
    #[error("duplicate signal source: {0}")]
    DuplicateSignalSource(QcIndexing),

    /// One source failed.
    #[error("source {indexing}: {error}")]
    Source {
        indexing: QcIndexing,
        error: QcSourceError,
    },
}

/// Errors raised by [QcEngine::run].
#[derive(Debug, Error)]
pub enum QcRunError<E: std::fmt::Debug> {
    #[error("engine: {0}")]
    Engine(#[from] QcEngineError),

    #[error("sink: {0:?}")]
    Sink(E),
}

/// Session wide (non temporal) data, available to all processors at all times.
#[derive(Debug, Default, Clone)]
pub struct QcStatics {
    /// Antenna calibration databases (ANTEX)
    pub antex: HashMap<QcIndexing, Rinex>,
}

/// All signals sampled by one receiver at the epoch being processed.
#[derive(Debug, Clone)]
pub struct QcEpochSignals {
    /// Signal source identity
    pub receiver: QcIndexing,

    /// Sampling conditions
    pub flag: EpochFlag,

    /// Receiver clock state, if provided
    pub clock: Option<ClockObservation>,

    /// Signal observations
    pub signals: Vec<SignalObservation>,
}

/// Everything a processor may use to process one epoch.
pub struct QcEpochContext<'a> {
    /// Epoch being processed
    pub epoch: Epoch,

    /// Receivers that sampled something at this epoch
    pub receivers: &'a [QcEpochSignals],

    /// Radio ephemerides, synchronized to this epoch
    pub ephemerides: &'a QcEphemerisStore,

    /// Session wide data
    pub statics: &'a QcStatics,
}

/// [QcProcessor] is implemented by every algorithm the engine runs.
/// Processors consume a synchronized [QcEpochContext] and produce results
/// into the single output stream.
pub trait QcProcessor<O = QcOutput> {
    /// Name of this processor (for logs)
    fn name(&self) -> &str;

    /// Process one synchronized epoch.
    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>);

    /// Called once, when all signal sources have ended.
    fn finalize(&mut self, _output: &mut QcOutputQueue<O>) {}
}

/// Engine statistics
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct QcEngineStats {
    /// Number of epochs processed
    pub epochs: usize,

    /// Number of input items pulled
    pub items: usize,

    /// Observations dropped because they arrived after their epoch was processed
    pub late_items: usize,

    /// Items not consumed by the engine (yet)
    pub unhandled_items: usize,
}

enum RxItem {
    Signal(QcSignal),
    Clock(ClockObservation),
}

struct SourceSlot {
    descriptor: QcSourceDescriptor,
    source: Box<dyn QcSource>,
    watermark: Option<Epoch>,
    eos: bool,
    observations: std::collections::VecDeque<(Epoch, RxItem)>,
}

enum Pull {
    Item,
    Pending,
    EndOfStream,
}

impl SourceSlot {
    fn front_epoch(&self) -> Option<Epoch> {
        self.observations.front().map(|(t, _)| *t)
    }

    /// Pulls one item and routes it.
    fn pull(
        &mut self,
        ephemerides: &mut QcEphemerisStore,
        last_epoch: Option<Epoch>,
        stats: &mut QcEngineStats,
    ) -> Result<Pull, QcEngineError> {
        if self.eos {
            return Ok(Pull::EndOfStream);
        }

        let item = match self.source.poll() {
            Ok(QcPoll::Ready(item)) => item,
            Ok(QcPoll::Pending) => return Ok(Pull::Pending),
            Ok(QcPoll::EndOfStream) => {
                #[cfg(feature = "log")]
                debug!("{}: end of stream", self.descriptor);
                self.eos = true;
                return Ok(Pull::EndOfStream);
            },
            Err(error) => {
                return Err(QcEngineError::Source {
                    indexing: self.descriptor.indexing.clone(),
                    error,
                })
            },
        };

        stats.items += 1;

        let QcInputItem { epoch, payload } = item;

        match self.watermark {
            Some(w) if epoch < w => {
                #[cfg(feature = "log")]
                warn!(
                    "{}: source is not time ordered ({} < {})",
                    self.descriptor, epoch, w
                );
            },
            _ => self.watermark = Some(epoch),
        }

        let is_late = last_epoch.is_some_and(|last| epoch <= last);

        match payload {
            QcPayload::Signal(signal) => {
                self.push_observation(epoch, RxItem::Signal(signal), is_late, stats)
            },
            QcPayload::ReceiverClock(clock) => {
                self.push_observation(epoch, RxItem::Clock(clock), is_late, stats)
            },
            QcPayload::Ephemeris(eph) => ephemerides.push(&self.descriptor.indexing, eph),
            QcPayload::PreciseState(_) => {
                // TODO: precise states buffering & interpolation (SP3 patch)
                stats.unhandled_items += 1;
            },
        }

        Ok(Pull::Item)
    }

    fn push_observation(
        &mut self,
        epoch: Epoch,
        item: RxItem,
        is_late: bool,
        stats: &mut QcEngineStats,
    ) {
        if is_late {
            stats.late_items += 1;
            return;
        }

        match self.observations.back() {
            Some((last, _)) if *last > epoch => {
                let pos = self.observations.partition_point(|(t, _)| *t <= epoch);
                self.observations.insert(pos, (epoch, item));
            },
            _ => self.observations.push_back((epoch, item)),
        }
    }

    fn is_signal_source(&self) -> bool {
        self.descriptor.signals
    }
}

/// Result of one processing step
enum Step {
    /// One epoch was processed (results may or may not have been produced)
    Processed,
    /// Waiting on a real-time source
    Pending,
    /// All signal sources ended, processors were finalized
    Done,
}

/// [QcEngineBuilder] collects the sources, processors and static data.
pub struct QcEngineBuilder<O = QcOutput> {
    sources: Vec<Box<dyn QcSource>>,
    processors: Vec<Box<dyn QcProcessor<O>>>,
    statics: QcStatics,
    lookahead: Duration,
    idle_sleep: StdDuration,
}

impl<O> Default for QcEngineBuilder<O> {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            processors: Vec::new(),
            statics: Default::default(),
            lookahead: Duration::from_hours(2.0),
            idle_sleep: StdDuration::from_millis(1),
        }
    }
}

impl<O> QcEngineBuilder<O> {
    /// Adds a new input [QcSource].
    pub fn with_source<S: QcSource + 'static>(mut self, source: S) -> Self {
        self.sources.push(Box::new(source));
        self
    }

    /// Adds a new (boxed) input [QcSource].
    pub fn with_boxed_source(mut self, source: Box<dyn QcSource>) -> Self {
        self.sources.push(source);
        self
    }

    /// Adds a new [QcProcessor]. Processors run in insertion order.
    pub fn with_processor<P: QcProcessor<O> + 'static>(mut self, processor: P) -> Self {
        self.processors.push(Box::new(processor));
        self
    }

    /// Adds an ANTEX database (session wide data).
    pub fn with_antex(mut self, indexing: QcIndexing, antex: Rinex) -> Self {
        self.statics.antex.insert(indexing, antex);
        self
    }

    /// Defines how far ahead of the epoch being processed context sources are pulled.
    /// Default is 2 hours, which suits radio ephemerides.
    pub fn with_lookahead(mut self, lookahead: Duration) -> Self {
        self.lookahead = lookahead;
        self
    }

    /// Defines how long the blocking [Iterator] sleeps while waiting on real-time sources.
    pub fn with_idle_sleep(mut self, idle: StdDuration) -> Self {
        self.idle_sleep = idle;
        self
    }

    /// Builds the [QcEngine].
    pub fn build(self) -> Result<QcEngine<O>, QcEngineError> {
        let mut slots: Vec<SourceSlot> = Vec::with_capacity(self.sources.len());

        for source in self.sources {
            let descriptor = source.descriptor().clone();

            if descriptor.signals
                && slots
                    .iter()
                    .any(|s| s.descriptor.signals && s.descriptor.indexing == descriptor.indexing)
            {
                return Err(QcEngineError::DuplicateSignalSource(descriptor.indexing));
            }

            slots.push(SourceSlot {
                descriptor,
                source,
                watermark: None,
                eos: false,
                observations: Default::default(),
            });
        }

        let nb_receivers = slots.iter().filter(|s| s.is_signal_source()).count();

        if nb_receivers == 0 {
            return Err(QcEngineError::NoSignalSource);
        }

        Ok(QcEngine {
            slots,
            processors: self.processors,
            statics: self.statics,
            lookahead: self.lookahead,
            idle_sleep: self.idle_sleep,
            ephemerides: Default::default(),
            output: Default::default(),
            epoch_signals: Vec::with_capacity(nb_receivers),
            last_epoch: None,
            finalized: false,
            stats: Default::default(),
        })
    }
}

/// The MISO processing engine. See the [module](crate::engine) documentation.
pub struct QcEngine<O = QcOutput> {
    slots: Vec<SourceSlot>,
    processors: Vec<Box<dyn QcProcessor<O>>>,
    statics: QcStatics,
    lookahead: Duration,
    idle_sleep: StdDuration,
    ephemerides: QcEphemerisStore,
    output: QcOutputQueue<O>,
    epoch_signals: Vec<QcEpochSignals>,
    last_epoch: Option<Epoch>,
    finalized: bool,
    stats: QcEngineStats,
}

impl<O> QcEngine<O> {
    /// Creates a new [QcEngineBuilder].
    pub fn builder() -> QcEngineBuilder<O> {
        QcEngineBuilder::default()
    }

    /// Engine statistics so far
    pub fn stats(&self) -> QcEngineStats {
        self.stats
    }

    /// Last processed [Epoch]
    pub fn last_epoch(&self) -> Option<Epoch> {
        self.last_epoch
    }

    /// Ephemerides currently buffered
    pub fn ephemerides(&self) -> &QcEphemerisStore {
        &self.ephemerides
    }

    /// Non blocking: returns the next result if one is available,
    /// [QcPoll::Pending] if waiting on a real-time source,
    /// or [QcPoll::EndOfStream] once everything has been processed.
    /// This is the method to integrate the engine in an event loop or an async runtime.
    pub fn poll_next(&mut self) -> Result<QcPoll<QcOutputItem<O>>, QcEngineError> {
        loop {
            if let Some(item) = self.output.pop() {
                return Ok(QcPoll::Ready(item));
            }

            match self.step()? {
                Step::Processed => continue,
                Step::Pending => return Ok(QcPoll::Pending),
                Step::Done => {
                    return Ok(match self.output.pop() {
                        Some(item) => QcPoll::Ready(item),
                        None => QcPoll::EndOfStream,
                    })
                },
            }
        }
    }

    /// Blocking: streams all results into the [QcSink], until all signal sources end.
    pub fn run<S: QcSink<O>>(self, sink: &mut S) -> Result<QcEngineStats, QcRunError<S::Error>>
    where
        S::Error: std::fmt::Debug,
    {
        let mut engine = self;

        for item in engine.by_ref() {
            sink.consume(item?).map_err(QcRunError::Sink)?;
        }

        Ok(engine.stats)
    }

    fn step(&mut self) -> Result<Step, QcEngineError> {
        if self.finalized {
            return Ok(Step::Done);
        }

        let Self {
            slots,
            processors,
            statics,
            lookahead,
            ephemerides,
            output,
            epoch_signals,
            last_epoch,
            stats,
            finalized,
            ..
        } = self;

        // 1. each live signal source must expose its next observation
        for slot in slots.iter_mut().filter(|s| s.is_signal_source()) {
            while slot.observations.is_empty() && !slot.eos {
                if let Pull::Pending = slot.pull(ephemerides, *last_epoch, stats)? {
                    return Ok(Step::Pending);
                }
            }
        }

        // 2. next epoch to process
        let t = match slots.iter().filter_map(|s| s.front_epoch()).min() {
            Some(t) => t,
            None => {
                for processor in processors.iter_mut() {
                    processor.finalize(output);
                }
                *finalized = true;
                return Ok(Step::Done);
            },
        };

        // 3. epoch must be complete for every live signal source
        for slot in slots.iter_mut().filter(|s| s.is_signal_source()) {
            while !slot.eos && slot.watermark.is_none_or(|w| w <= t) {
                if let Pull::Pending = slot.pull(ephemerides, *last_epoch, stats)? {
                    return Ok(Step::Pending);
                }
            }
        }

        // 4. context sources must cover t + lookahead (never blocking)
        let horizon = t + *lookahead;

        for slot in slots.iter_mut().filter(|s| s.descriptor.provides_context()) {
            while !slot.eos && slot.watermark.is_none_or(|w| w <= horizon) {
                match slot.pull(ephemerides, *last_epoch, stats)? {
                    Pull::Item => {},
                    Pull::Pending | Pull::EndOfStream => break,
                }
            }
        }

        ephemerides.prune(t);

        // 5. gather this epoch, per receiver (buffers are reused)
        let mut nb_active = 0;

        for slot in slots.iter_mut() {
            if slot.front_epoch() != Some(t) {
                continue;
            }

            if nb_active == epoch_signals.len() {
                epoch_signals.push(QcEpochSignals {
                    receiver: slot.descriptor.indexing.clone(),
                    flag: EpochFlag::default(),
                    clock: None,
                    signals: Vec::with_capacity(64),
                });
            }

            let entry = &mut epoch_signals[nb_active];

            if entry.receiver != slot.descriptor.indexing {
                entry.receiver = slot.descriptor.indexing.clone();
            }

            entry.clock = None;
            entry.signals.clear();

            let mut first = true;

            while slot.front_epoch() == Some(t) {
                let (_, item) = slot.observations.pop_front().unwrap();
                match item {
                    RxItem::Signal(signal) => {
                        if first {
                            entry.flag = signal.flag;
                            first = false;
                        }
                        entry.signals.push(signal.signal);
                    },
                    RxItem::Clock(clock) => entry.clock = Some(clock),
                }
            }

            nb_active += 1;
        }

        *last_epoch = Some(t);
        stats.epochs += 1;

        // 6. run processors
        let ctx = QcEpochContext {
            epoch: t,
            receivers: &epoch_signals[..nb_active],
            ephemerides,
            statics,
        };

        for processor in processors.iter_mut() {
            processor.process(&ctx, output);
        }

        Ok(Step::Processed)
    }
}

/// Blocking [Iterator] over the results. While waiting on real-time sources,
/// the calling thread sleeps (see [QcEngineBuilder::with_idle_sleep]).
/// Use [QcEngine::poll_next] for non blocking integration.
impl<O> Iterator for QcEngine<O> {
    type Item = Result<QcOutputItem<O>, QcEngineError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.poll_next() {
                Ok(QcPoll::Ready(item)) => return Some(Ok(item)),
                Ok(QcPoll::Pending) => std::thread::sleep(self.idle_sleep),
                Ok(QcPoll::EndOfStream) => return None,
                Err(e) => return Some(Err(e)),
            }
        }
    }
}
