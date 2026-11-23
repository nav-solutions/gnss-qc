//! Engine output: a single, technology agnostic, stream of results.
//!
//! The engine is MISO (Multiple Inputs, Single Output). It is up to the application
//! to consume this stream: render it, serialize it, forward it, or process it further.
//!
//! The output type is generic: the built-in processors produce [QcOutput], and any
//! application specific output type `O: From<QcOutput>` may be used to stack custom
//! processors next to the built-in ones.
use std::{collections::VecDeque, convert::Infallible, sync::mpsc::Sender};

use gnss_rs::prelude::SV;
use hifitime::Epoch;

use rinex::{
    navigation::Ephemeris,
    prelude::obs::{EpochFlag, SignalObservation},
};

use crate::io::input::indexing::QcIndexing;

#[cfg(feature = "serde")]
use serde::Serialize;

/// Ephemeris coverage of a signal source.
/// Emitted on every change: coverage gained, coverage lost, or switch to another ephemeris.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct QcEphemerisCoverage {
    /// Receiver (signal source) being analyzed
    pub receiver: QcIndexing,

    /// Satellite in sight
    pub sv: SV,

    /// Source of the ephemeris that is now being used, None if not covered.
    pub source: Option<QcIndexing>,

    /// ToC of the ephemeris that is now being used, None if not covered.
    pub toc: Option<Epoch>,
}

/// Signal observation, streamed out as is.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct QcSignalOutput {
    /// Receiver (signal source)
    pub receiver: QcIndexing,

    /// Sampling conditions
    pub flag: EpochFlag,

    /// Observation
    pub signal: SignalObservation,
}

/// Radio ephemeris, streamed out as is.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct QcEphemerisOutput {
    /// Ephemeris source
    pub source: QcIndexing,

    /// Satellite
    pub sv: SV,

    /// Time of Clock
    pub toc: Epoch,

    /// Ephemeris frame
    pub ephemeris: Box<Ephemeris>,
}

/// Results produced by the built-in processors.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum QcOutput {
    /// Signal observation (passthrough)
    Signal(QcSignalOutput),

    /// Ephemeris coverage change
    EphemerisCoverage(QcEphemerisCoverage),

    /// Radio ephemeris (passthrough)
    Ephemeris(QcEphemerisOutput),
}

/// Time tagged result.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct QcOutputItem<O = QcOutput> {
    /// [Epoch] this result applies to
    pub epoch: Epoch,

    /// The result itself
    pub payload: O,
}

/// Output queue the processors push their results into.
/// The queue is drained by the engine consumer, which keeps it small.
#[derive(Debug)]
pub struct QcOutputQueue<O = QcOutput> {
    inner: VecDeque<QcOutputItem<O>>,
}

impl<O> Default for QcOutputQueue<O> {
    fn default() -> Self {
        Self {
            inner: VecDeque::with_capacity(64),
        }
    }
}

impl<O> QcOutputQueue<O> {
    /// Push a new result
    pub fn push<T: Into<O>>(&mut self, epoch: Epoch, payload: T) {
        self.inner.push_back(QcOutputItem {
            epoch,
            payload: payload.into(),
        });
    }

    pub(crate) fn pop(&mut self) -> Option<QcOutputItem<O>> {
        self.inner.pop_front()
    }

    /// Number of results waiting to be consumed
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if no results are waiting
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

/// [QcSink] is the generic consumer of the output stream.
/// It is implemented for [Vec], [Sender] and closures, implement it for anything else
/// (file writer, socket, database..).
pub trait QcSink<O = QcOutput> {
    type Error;

    /// Consume one result.
    fn consume(&mut self, item: QcOutputItem<O>) -> Result<(), Self::Error>;
}

impl<O> QcSink<O> for Vec<QcOutputItem<O>> {
    type Error = Infallible;

    fn consume(&mut self, item: QcOutputItem<O>) -> Result<(), Self::Error> {
        self.push(item);
        Ok(())
    }
}

impl<O> QcSink<O> for Sender<QcOutputItem<O>> {
    type Error = std::sync::mpsc::SendError<QcOutputItem<O>>;

    fn consume(&mut self, item: QcOutputItem<O>) -> Result<(), Self::Error> {
        self.send(item)
    }
}

/// Closure based [QcSink].
pub struct QcFnSink<F>(pub F);

impl<O, E, F: FnMut(QcOutputItem<O>) -> Result<(), E>> QcSink<O> for QcFnSink<F> {
    type Error = E;

    fn consume(&mut self, item: QcOutputItem<O>) -> Result<(), E> {
        (self.0)(item)
    }
}
