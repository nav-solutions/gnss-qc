//! Input sources.
//!
//! A [QcSource] is anything that can provide time-tagged data points
//! ([QcInputItem]): a RINEX file, an SP3 file, a socket, a serial port,
//! a channel fed by another thread...
//!
//! Design principles:
//! - **pull based**: the engine pulls one item at a time, when it needs it.
//!   No source is ever fully loaded by the engine itself, which keeps memory bounded.
//! - **runtime agnostic**: no async runtime is required. A source that has
//!   nothing to deliver *yet* (real-time socket) returns [QcPoll::Pending]
//!   instead of blocking. Files never return [QcPoll::Pending].
//! - **multi nature**: any source may deliver any kind of [QcPayload]
//!   (an RTCM stream carries both observations and ephemerides).
//!   [QcSourceDescriptor] simply tells the engine what to expect, so it
//!   knows which sources it must wait for.
//! - **time ordered**: each source must deliver items in chronological order
//!   (non decreasing [QcInputItem::epoch]). Late items are dropped by the engine.
use std::{
    fmt,
    sync::mpsc::{Receiver, TryRecvError},
};

use gnss_rs::prelude::SV;
use hifitime::Epoch;

use rinex::{
    navigation::Ephemeris,
    prelude::obs::{ClockObservation, EpochFlag, SignalObservation},
};

use crate::io::input::indexing::QcIndexing;

/// Boxed error type returned by [QcSource]s. Boxed to keep [QcSource] object safe,
/// so sources of different natures can be stacked in the same engine.
pub type QcSourceError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Result of a non blocking poll operation.
#[derive(Debug, Clone, PartialEq)]
pub enum QcPoll<T> {
    /// One item is available.
    Ready(T),

    /// Nothing available right now, but the stream is still alive.
    /// Only real-time sources should return this.
    Pending,

    /// This stream is over and will never deliver anything else.
    EndOfStream,
}

/// One signal observation, sampled by a receiver.
#[derive(Debug, Clone, PartialEq)]
pub struct QcSignal {
    /// Sampling conditions
    pub flag: EpochFlag,

    /// The observation itself (satellite, observable, value, LLI, SNR)
    pub signal: SignalObservation,
}

/// One radio (broadcast) ephemeris.
#[derive(Debug, Clone, PartialEq)]
pub struct QcEphemeris {
    /// Broadcasting satellite
    pub sv: SV,

    /// Time of Clock
    pub toc: Epoch,

    /// The ephemeris frame. Boxed because this structure is large
    /// compared to the other payloads.
    pub ephemeris: Box<Ephemeris>,
}

impl QcEphemeris {
    /// Returns Time of Ephemeris, when it can be determined.
    pub fn toe(&self) -> Option<Epoch> {
        self.ephemeris.toe(self.sv)
    }

    /// Returns true if this ephemeris may be used at `t`.
    /// When the ToE cannot be determined (Glonass for example),
    /// the validity period is applied around the ToC.
    pub fn is_valid(&self, t: Epoch) -> bool {
        match self.toe() {
            Some(_) => self.ephemeris.is_valid(self.sv, t),
            None => match Ephemeris::validity_duration(self.sv.constellation) {
                Some(dt) => (t - self.toc).abs() < dt,
                None => false,
            },
        }
    }
}

/// Precise satellite state (typically from SP3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QcPreciseState {
    /// Satellite
    pub sv: SV,

    /// Position in km, ECEF
    pub position_km: (f64, f64, f64),

    /// Possible clock offset, in seconds
    pub clock_offset_s: Option<f64>,
}

/// All data points the engine knows how to process.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum QcPayload {
    /// Signal observation (raw measurement)
    Signal(QcSignal),

    /// Receiver clock state
    ReceiverClock(ClockObservation),

    /// Radio ephemeris
    Ephemeris(QcEphemeris),

    /// Precise satellite state
    PreciseState(QcPreciseState),
}

/// Time tagged data point, delivered by a [QcSource].
#[derive(Debug, Clone, PartialEq)]
pub struct QcInputItem {
    /// Ordering [Epoch]: sampling epoch for observations,
    /// ToC for ephemerides, state epoch for precise states.
    pub epoch: Epoch,

    /// Data point
    pub payload: QcPayload,
}

impl QcInputItem {
    /// Builds a new signal observation item.
    pub fn signal(epoch: Epoch, flag: EpochFlag, signal: SignalObservation) -> Self {
        Self {
            epoch,
            payload: QcPayload::Signal(QcSignal { flag, signal }),
        }
    }

    /// Builds a new ephemeris item (ordered by ToC).
    pub fn ephemeris(toc: Epoch, sv: SV, ephemeris: Ephemeris) -> Self {
        Self {
            epoch: toc,
            payload: QcPayload::Ephemeris(QcEphemeris {
                sv,
                toc,
                ephemeris: Box::new(ephemeris),
            }),
        }
    }

    /// Builds a new receiver clock item.
    pub fn receiver_clock(epoch: Epoch, clock: ClockObservation) -> Self {
        Self {
            epoch,
            payload: QcPayload::ReceiverClock(clock),
        }
    }
}

/// What a source will deliver. The engine uses this to know which sources it must
/// wait for before processing an epoch:
/// - sources providing signals must have moved past the epoch being processed
///   (otherwise the epoch might be incomplete);
/// - sources providing context data (ephemerides, precise states) must cover the
///   epoch being processed plus the lookahead window.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QcSourceDescriptor {
    /// How this source is identified (receiver, agency..).
    /// Data points are stored and presented to the processors with this identity.
    pub indexing: QcIndexing,

    /// This source delivers signal observations
    pub signals: bool,

    /// This source delivers radio ephemerides
    pub ephemerides: bool,

    /// This source delivers precise states
    pub precise_states: bool,
}

impl QcSourceDescriptor {
    /// New descriptor that does not provide anything yet.
    pub fn new(indexing: QcIndexing) -> Self {
        Self {
            indexing,
            signals: false,
            ephemerides: false,
            precise_states: false,
        }
    }

    /// Signal source (observation RINEX, receiver stream..)
    pub fn with_signals(mut self) -> Self {
        self.signals = true;
        self
    }

    /// Ephemeris source (navigation RINEX..)
    pub fn with_ephemerides(mut self) -> Self {
        self.ephemerides = true;
        self
    }

    /// Precise states source (SP3..)
    pub fn with_precise_states(mut self) -> Self {
        self.precise_states = true;
        self
    }

    /// Returns true if this source provides context data
    /// (anything that is not an observation).
    pub fn provides_context(&self) -> bool {
        self.ephemerides || self.precise_states
    }
}

impl fmt::Display for QcSourceDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.indexing)
    }
}

/// [QcSource] must be implemented by all inputs of the engine.
pub trait QcSource {
    /// Describes this source.
    fn descriptor(&self) -> &QcSourceDescriptor;

    /// Pulls the next [QcInputItem], without blocking.
    fn poll(&mut self) -> Result<QcPoll<QcInputItem>, QcSourceError>;
}

/// Wraps any [Iterator] of [QcInputItem] into a [QcSource].
/// Never pending: this is the natural adapter for files and in-memory datasets.
pub struct QcIterSource<I: Iterator<Item = QcInputItem>> {
    descriptor: QcSourceDescriptor,
    iter: I,
}

impl<I: Iterator<Item = QcInputItem>> QcIterSource<I> {
    pub fn new(descriptor: QcSourceDescriptor, iter: I) -> Self {
        Self { descriptor, iter }
    }
}

impl<I: Iterator<Item = QcInputItem>> QcSource for QcIterSource<I> {
    fn descriptor(&self) -> &QcSourceDescriptor {
        &self.descriptor
    }

    fn poll(&mut self) -> Result<QcPoll<QcInputItem>, QcSourceError> {
        Ok(match self.iter.next() {
            Some(item) => QcPoll::Ready(item),
            None => QcPoll::EndOfStream,
        })
    }
}

/// Real-time source fed by another thread (socket reader, serial port, decoder...)
/// through a standard [std::sync::mpsc] channel.
/// Returns [QcPoll::Pending] when the channel is empty,
/// and [QcPoll::EndOfStream] when all senders have been dropped.
pub struct QcChannelSource {
    descriptor: QcSourceDescriptor,
    receiver: Receiver<QcInputItem>,
}

impl QcChannelSource {
    pub fn new(descriptor: QcSourceDescriptor, receiver: Receiver<QcInputItem>) -> Self {
        Self {
            descriptor,
            receiver,
        }
    }
}

impl QcSource for QcChannelSource {
    fn descriptor(&self) -> &QcSourceDescriptor {
        &self.descriptor
    }

    fn poll(&mut self) -> Result<QcPoll<QcInputItem>, QcSourceError> {
        Ok(match self.receiver.try_recv() {
            Ok(item) => QcPoll::Ready(item),
            Err(TryRecvError::Empty) => QcPoll::Pending,
            Err(TryRecvError::Disconnected) => QcPoll::EndOfStream,
        })
    }
}
