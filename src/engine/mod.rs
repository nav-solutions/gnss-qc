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
//!
//! When no source provides signals (context only session, for example
//! a navigation file alone), epochs are driven by the context data points themselves:
//! the epochs of the data points ingested so far, with the same lookahead.
use std::{collections::HashMap, time::Duration as StdDuration};

use hifitime::{Duration, Epoch};

use rinex::prelude::{
    obs::{ClockObservation, EpochFlag, SignalObservation},
    Rinex,
};

use thiserror::Error;

#[cfg(feature = "log")]
use log::{debug, info, warn};

use crate::{
    io::{
        input::{
            indexing::QcIndexing,
            product::{QcInputProducts, QcProductSources},
        },
        output::{QcOutput, QcOutputItem, QcOutputQueue, QcSink},
        source::{
            QcInputItem, QcPayload, QcPoll, QcSignal, QcSource, QcSourceDescriptor, QcSourceError,
        },
    },
    sync::{QcClockStore, QcEphemerisStore, QcMeteoStore, QcPreciseStore, QcTecStore},
};

/// Context data stores, synchronized by the engine
#[derive(Debug, Default)]
struct QcStores {
    ephemerides: QcEphemerisStore,
    precise_states: QcPreciseStore,
    precise_clocks: QcClockStore,
    meteo: QcMeteoStore,
    tec_maps: QcTecStore,
}

impl QcStores {
    fn prune(&mut self, t: Epoch) {
        self.ephemerides.prune(t);
        self.precise_states.prune(t);
        self.precise_clocks.prune(t);
        self.meteo.prune(t);
        self.tec_maps.prune(t);
    }
}

/// Errors raised by the engine.
#[derive(Debug, Error)]
pub enum QcEngineError {
    /// At least one source is required.
    #[error("no input source")]
    NoSource,

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

    /// Reference position of the antenna (ECEF m), if known (see [QcSourceDescriptor])
    pub reference_position_ecef_m: Option<(f64, f64, f64)>,

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

    /// Precise states, synchronized to this epoch (interpolation window)
    pub precise_states: &'a QcPreciseStore,

    /// Precise clock states (satellites and ground stations),
    /// synchronized to this epoch (interpolation window)
    pub precise_clocks: &'a QcClockStore,

    /// Meteo observations, synchronized to this epoch
    pub meteo: &'a QcMeteoStore,

    /// TEC maps, synchronized to this epoch (surrounding maps)
    pub tec_maps: &'a QcTecStore,

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QcEngineStats {
    /// Number of epochs processed
    pub epochs: usize,

    /// Number of input items pulled
    pub items: usize,

    /// Observations dropped because they arrived after their epoch was processed
    pub late_items: usize,

    /// Items not consumed by the engine (yet)
    pub unhandled_items: usize,

    /// Epochs processed without one (or more) lagging signal source
    /// ([QcEngineBuilder::with_max_latency])
    pub incomplete_epochs: usize,

    /// Sources that failed and were ended ([QcEngineBuilder::with_fault_tolerance])
    pub failed_sources: usize,
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
    /// A failure ends this source, instead of the session
    tolerant: bool,
    /// This source failed (and ended)
    failed: bool,
    /// This source is lagging (not waited for)
    lagging: bool,
    peeked: Option<QcInputItem>,
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

    /// Polls the next raw item (peeked item first).
    fn poll_raw(&mut self) -> Result<QcPoll<QcInputItem>, QcEngineError> {
        if let Some(item) = self.peeked.take() {
            return Ok(QcPoll::Ready(item));
        }

        if self.eos {
            return Ok(QcPoll::EndOfStream);
        }

        match self.source.poll() {
            Ok(QcPoll::EndOfStream) => {
                #[cfg(feature = "log")]
                debug!("{}: end of stream", self.descriptor);
                self.eos = true;
                Ok(QcPoll::EndOfStream)
            }
            Ok(other) => Ok(other),
            Err(error) if self.tolerant => {
                #[cfg(feature = "log")]
                warn!("{}: {} (source ended)", self.descriptor, error);
                #[cfg(not(feature = "log"))]
                let _ = error;
                self.eos = true;
                self.failed = true;
                Ok(QcPoll::EndOfStream)
            }
            Err(error) => Err(QcEngineError::Source {
                indexing: self.descriptor.indexing.clone(),
                error,
            }),
        }
    }

    /// Epoch of the next item, without consuming it.
    fn peek(&mut self) -> Result<QcPoll<Epoch>, QcEngineError> {
        if self.peeked.is_none() {
            match self.poll_raw()? {
                QcPoll::Ready(item) => self.peeked = Some(item),
                QcPoll::Pending => return Ok(QcPoll::Pending),
                QcPoll::EndOfStream => return Ok(QcPoll::EndOfStream),
            }
        }
        Ok(QcPoll::Ready(self.peeked.as_ref().unwrap().epoch))
    }

    /// Pulls one item and routes it.
    fn pull(
        &mut self,
        stores: &mut QcStores,
        last_epoch: Option<Epoch>,
        stats: &mut QcEngineStats,
    ) -> Result<Pull, QcEngineError> {
        let item = match self.poll_raw()? {
            QcPoll::Ready(item) => item,
            QcPoll::Pending => return Ok(Pull::Pending),
            QcPoll::EndOfStream => return Ok(Pull::EndOfStream),
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
            }
            _ => self.watermark = Some(epoch),
        }

        let is_late = last_epoch.is_some_and(|last| epoch <= last);

        match payload {
            QcPayload::Signal(signal) => {
                self.push_observation(epoch, RxItem::Signal(signal), is_late, stats)
            }
            QcPayload::ReceiverClock(clock) => {
                self.push_observation(epoch, RxItem::Clock(clock), is_late, stats)
            }
            QcPayload::Ephemeris(eph) => stores.ephemerides.push(&self.descriptor.indexing, eph),
            QcPayload::PreciseState(state) => {
                stores.precise_states.push(&self.descriptor.indexing, state)
            }
            QcPayload::PreciseClock(clock) => {
                stores.precise_clocks.push(&self.descriptor.indexing, clock)
            }
            QcPayload::Meteo(sample) => stores.meteo.push(&self.descriptor.indexing, epoch, sample),
            QcPayload::Tec(point) => stores
                .tec_maps
                .push(&self.descriptor.indexing, epoch, point),
            QcPayload::ReferencePosition(position_ecef_m) => {
                if self.descriptor.reference_position_ecef_m != Some(position_ecef_m) {
                    #[cfg(feature = "log")]
                    debug!(
                        "{}: reference position {:?} (ECEF m)",
                        self.descriptor, position_ecef_m
                    );
                    self.descriptor.reference_position_ecef_m = Some(position_ecef_m);
                }
            }
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
            }
            _ => self.observations.push_back((epoch, item)),
        }
    }

    fn is_signal_source(&self) -> bool {
        self.descriptor.signals
    }

    /// Updates the lagging state of this source (logged on change)
    fn set_lagging(&mut self, lagging: bool) {
        #[cfg(feature = "log")]
        if lagging != self.lagging {
            if lagging {
                warn!(
                    "{}: lagging (latest data: {}), not waited for",
                    self.descriptor,
                    self.watermark
                        .map(|w| w.to_string())
                        .unwrap_or("none".to_string())
                );
            } else {
                info!("{}: caught up", self.descriptor);
            }
        }
        self.lagging = lagging;
    }
}

/// Real-time: returns true if the (pending) live signal source `index` lags behind the other
/// signal sources by more than `max_latency`. The other sources are pulled ahead (at most
/// `max_latency` beyond the lagging source) to decide.
fn is_lagging(
    slots: &mut [SourceSlot],
    index: usize,
    max_latency: Option<Duration>,
    first_signal: &mut Option<Epoch>,
    stores: &mut QcStores,
    last_epoch: Option<Epoch>,
    stats: &mut QcEngineStats,
) -> Result<bool, QcEngineError> {
    let Some(latency) = max_latency else {
        return Ok(false);
    };

    let horizon = slots[index]
        .watermark
        .or(*first_signal)
        .map(|t| t + latency);

    for (j, slot) in slots.iter_mut().enumerate() {
        if j == index || !slot.is_signal_source() {
            continue;
        }
        loop {
            let ahead = match (slot.watermark, horizon) {
                (Some(w), Some(h)) => w > h,
                (Some(_), None) => true, // first data of the session: one item is enough
                (None, _) => false,
            };
            if ahead || slot.eos {
                break;
            }
            match slot.pull(stores, last_epoch, stats)? {
                Pull::Item => {}
                Pull::Pending | Pull::EndOfStream => break,
            }
        }
    }

    let oldest = slots
        .iter()
        .filter(|s| s.is_signal_source())
        .filter_map(|s| s.watermark)
        .min();

    if first_signal.is_none() {
        *first_signal = oldest;
    }

    let newest = slots
        .iter()
        .filter(|s| s.is_signal_source())
        .filter_map(|s| s.watermark)
        .max();

    let reference = slots[index].watermark.or(*first_signal);

    let lagging = match (reference, newest) {
        (Some(reference), Some(newest)) => reference + latency < newest,
        _ => false,
    };

    slots[index].set_lagging(lagging);
    Ok(lagging)
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
    window: (Option<Epoch>, Option<Epoch>),
    max_latency: Option<Duration>,
    fault_tolerance: bool,
}

impl<O> Default for QcEngineBuilder<O> {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            processors: Vec::new(),
            statics: Default::default(),
            lookahead: Duration::from_hours(3.0),
            idle_sleep: StdDuration::from_millis(1),
            window: (None, None),
            max_latency: None,
            fault_tolerance: false,
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

    /// Adds all sources and session wide data obtained from a [QcInputProducts] table.
    pub fn with_products(mut self, products: QcInputProducts) -> Self {
        let QcProductSources { sources, antex } = products.into_sources();
        self.sources.extend(sources);
        self.statics.antex.extend(antex);
        self
    }

    /// Defines how far ahead of the epoch being processed context sources are pulled.
    /// Default is 3 hours: suits radio ephemerides, and precise products interpolation
    /// at the beginning of a file (order 10 at 15' sampling requires 2h30 of data).
    pub fn with_lookahead(mut self, lookahead: Duration) -> Self {
        self.lookahead = lookahead;
        self
    }

    /// Restricts the processing to [start, end] (both included, both optional).
    /// Epochs before `start` are consumed but not processed (context data is
    /// still synchronized), the session ends after `end`.
    pub fn with_time_window(mut self, start: Option<Epoch>, end: Option<Epoch>) -> Self {
        self.window = (start, end);
        self
    }

    /// Defines how long the blocking [Iterator] sleeps while waiting on real-time sources.
    pub fn with_idle_sleep(mut self, idle: StdDuration) -> Self {
        self.idle_sleep = idle;
        self
    }

    /// Real-time networks: a live signal source whose data lags behind the newest data of the
    /// session by more than `latency` (silent station, reconnecting client..) is no longer
    /// waited for. The epochs are processed without it ([QcEngineStats::incomplete_epochs]),
    /// its late observations are dropped ([QcEngineStats::late_items]), and it is waited for
    /// again as soon as it catches up. Default: every live signal source is waited for
    /// (files, recorded streams).
    pub fn with_max_latency(mut self, latency: Duration) -> Self {
        self.max_latency = Some(latency);
        self
    }

    /// A source that fails (connection failure, corrupted file..) is ended and the session
    /// continues with the other sources ([QcEngineStats::failed_sources]).
    /// Default: the failure of one source aborts the session ([QcEngineError::Source]).
    pub fn with_fault_tolerance(mut self, tolerant: bool) -> Self {
        self.fault_tolerance = tolerant;
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
                tolerant: self.fault_tolerance,
                failed: false,
                lagging: false,
                peeked: None,
                observations: Default::default(),
            });
        }

        let nb_receivers = slots.iter().filter(|s| s.is_signal_source()).count();

        if slots.is_empty() {
            return Err(QcEngineError::NoSource);
        }

        Ok(QcEngine {
            slots,
            processors: self.processors,
            statics: self.statics,
            lookahead: self.lookahead,
            idle_sleep: self.idle_sleep,
            window: self.window,
            max_latency: self.max_latency,
            first_signal: None,
            pending_epochs: Default::default(),
            stores: Default::default(),
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
    window: (Option<Epoch>, Option<Epoch>),
    max_latency: Option<Duration>,
    /// Oldest data of the signal sources (lagging sources without data)
    first_signal: Option<Epoch>,
    pending_epochs: std::collections::BTreeSet<Epoch>,
    stores: QcStores,
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
        let mut stats = self.stats;
        stats.failed_sources = self.slots.iter().filter(|s| s.failed).count();
        stats
    }

    /// Last processed [Epoch]
    pub fn last_epoch(&self) -> Option<Epoch> {
        self.last_epoch
    }

    /// Ephemerides currently buffered
    pub fn ephemerides(&self) -> &QcEphemerisStore {
        &self.stores.ephemerides
    }

    /// Precise states currently buffered
    pub fn precise_states(&self) -> &QcPreciseStore {
        &self.stores.precise_states
    }

    /// Precise clocks store (current synchronization state)
    pub fn precise_clocks(&self) -> &QcClockStore {
        &self.stores.precise_clocks
    }

    /// Meteo observations store (current synchronization state)
    pub fn meteo(&self) -> &QcMeteoStore {
        &self.stores.meteo
    }

    /// TEC maps store (current synchronization state)
    pub fn tec_maps(&self) -> &QcTecStore {
        &self.stores.tec_maps
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
                }
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

        if !self.slots.iter().any(|s| s.is_signal_source()) {
            return self.step_context_only();
        }

        // end of the time window: nothing else to process, even if some (real-time)
        // sources are still alive
        if let (Some(end), Some(last)) = (self.window.1, self.last_epoch) {
            if last >= end {
                for processor in self.processors.iter_mut() {
                    processor.finalize(&mut self.output);
                }
                self.finalized = true;
                return Ok(Step::Done);
            }
        }

        let Self {
            slots,
            processors,
            statics,
            lookahead,
            window,
            stores,
            output,
            epoch_signals,
            last_epoch,
            stats,
            finalized,
            max_latency,
            first_signal,
            ..
        } = self;

        let max_latency = *max_latency;
        let mut incomplete = false;

        // 1. each live signal source must expose its next observation
        //    (real-time: unless it is lagging)
        for index in 0..slots.len() {
            if !slots[index].is_signal_source() {
                continue;
            }
            while slots[index].observations.is_empty() && !slots[index].eos {
                if let Pull::Pending = slots[index].pull(stores, *last_epoch, stats)? {
                    if is_lagging(
                        slots,
                        index,
                        max_latency,
                        first_signal,
                        stores,
                        *last_epoch,
                        stats,
                    )? {
                        incomplete = true;
                        break;
                    }
                    return Ok(Step::Pending);
                }
            }
        }

        // 2. next epoch to process
        let t = match slots.iter().filter_map(|s| s.front_epoch()).min() {
            Some(t) if window.1.is_none_or(|end| t <= end) => t,
            _ => {
                for processor in processors.iter_mut() {
                    processor.finalize(output);
                }
                *finalized = true;
                return Ok(Step::Done);
            }
        };

        // 3. epoch must be complete for every live signal source (lagging sources excepted)
        for index in 0..slots.len() {
            if !slots[index].is_signal_source() {
                continue;
            }
            while !slots[index].eos && slots[index].watermark.is_none_or(|w| w <= t) {
                if let Pull::Pending = slots[index].pull(stores, *last_epoch, stats)? {
                    if is_lagging(
                        slots,
                        index,
                        max_latency,
                        first_signal,
                        stores,
                        *last_epoch,
                        stats,
                    )? {
                        incomplete = true;
                        break;
                    }
                    return Ok(Step::Pending);
                }
            }
        }

        // 4. context sources must cover t + lookahead (never blocking)
        let horizon = t + *lookahead;

        for slot in slots.iter_mut().filter(|s| s.descriptor.provides_context()) {
            while !slot.eos && slot.watermark.is_none_or(|w| w <= horizon) {
                match slot.pull(stores, *last_epoch, stats)? {
                    Pull::Item => {}
                    Pull::Pending | Pull::EndOfStream => break,
                }
            }
        }

        stores.prune(t);

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
                    reference_position_ecef_m: None,
                    signals: Vec::with_capacity(64),
                });
            }

            let entry = &mut epoch_signals[nb_active];

            if entry.receiver != slot.descriptor.indexing {
                entry.receiver = slot.descriptor.indexing.clone();
            }

            entry.clock = None;
            entry.reference_position_ecef_m = slot.descriptor.reference_position_ecef_m;
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
                    }
                    RxItem::Clock(clock) => entry.clock = Some(clock),
                }
            }

            nb_active += 1;
        }

        *last_epoch = Some(t);

        if window.0.is_some_and(|start| t < start) {
            return Ok(Step::Processed);
        }

        stats.epochs += 1;

        if incomplete {
            stats.incomplete_epochs += 1;
        }

        // 6. run processors
        let ctx = QcEpochContext {
            epoch: t,
            receivers: &epoch_signals[..nb_active],
            ephemerides: &stores.ephemerides,
            precise_states: &stores.precise_states,
            precise_clocks: &stores.precise_clocks,
            meteo: &stores.meteo,
            tec_maps: &stores.tec_maps,
            statics,
        };

        for processor in processors.iter_mut() {
            processor.process(&ctx, output);
        }

        Ok(Step::Processed)
    }
}

impl<O> QcEngine<O> {
    /// Context only session (no signal source, for example: ephemerides analysis).
    /// Epochs are driven by the context data points themselves.
    fn step_context_only(&mut self) -> Result<Step, QcEngineError> {
        let Self {
            slots,
            processors,
            statics,
            stores,
            output,
            last_epoch,
            stats,
            finalized,
            lookahead,
            window,
            pending_epochs,
            ..
        } = self;

        // 1. epochs to process: the epochs of the data points ingested so far, and the
        //    next data point of every source (whatever its distance: sources may have
        //    data gaps larger than the lookahead, or describe distinct periods)
        let mut pending = false;

        for slot in slots.iter_mut() {
            match slot.peek()? {
                QcPoll::Ready(epoch) => {
                    pending_epochs.insert(epoch);
                }
                QcPoll::Pending => pending = true,
                QcPoll::EndOfStream => {}
            }
        }

        if pending_epochs.is_empty() && pending {
            return Ok(Step::Pending);
        }

        let t = match pending_epochs.first() {
            Some(t) if window.1.is_none_or(|end| *t <= end) => *t,
            _ => {
                for processor in processors.iter_mut() {
                    processor.finalize(output);
                }
                *finalized = true;
                return Ok(Step::Done);
            }
        };

        // 2. context must cover t + lookahead (never blocking)
        let horizon = t + *lookahead;

        for slot in slots.iter_mut() {
            while let QcPoll::Ready(epoch) = slot.peek()? {
                if epoch > horizon {
                    break;
                }
                pending_epochs.insert(epoch);
                slot.pull(stores, *last_epoch, stats)?;
            }
        }

        pending_epochs.remove(&t);
        stores.prune(t);
        *last_epoch = Some(t);

        if window.0.is_some_and(|start| t < start) {
            return Ok(Step::Processed);
        }

        stats.epochs += 1;

        let ctx = QcEpochContext {
            epoch: t,
            receivers: &[],
            ephemerides: &stores.ephemerides,
            precise_states: &stores.precise_states,
            precise_clocks: &stores.precise_clocks,
            meteo: &stores.meteo,
            tec_maps: &stores.tec_maps,
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
