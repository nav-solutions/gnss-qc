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
use serde::{Deserialize, Serialize};

/// Ephemeris coverage of a signal source.
/// Emitted on every change: coverage gained, coverage lost, or switch to another ephemeris.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

/// P.V.T solution, resolved for one receiver
#[cfg(feature = "navigation")]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct QcPvtSolution {
    /// Receiver (rover)
    pub receiver: QcIndexing,

    /// Solution
    pub solution: gnss_rtk::prelude::PVTSolution,
}

#[cfg(feature = "navigation")]
impl PartialEq for QcPvtSolution {
    fn eq(&self, rhs: &Self) -> bool {
        self.receiver == rhs.receiver
            && self.solution.epoch == rhs.solution.epoch
            && self.solution.solution_type == rhs.solution.solution_type
            && self.solution.pos_m == rhs.solution.pos_m
            && self.solution.vel_m_s == rhs.solution.vel_m_s
            && self.solution.clock_offset_s == rhs.solution.clock_offset_s
    }
}

/// Navigation summary, emitted once all signal sources ended
#[cfg(feature = "navigation")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct QcNavigationReport {
    /// Receiver (rover)
    pub receiver: QcIndexing,

    /// Number of epochs presented to the solver
    pub epochs: usize,

    /// Last epoch presented to the solver
    pub last_epoch: Option<Epoch>,

    /// Number of solutions
    pub solutions: usize,

    /// Failures, per reason
    pub failures: std::collections::BTreeMap<String, usize>,

    /// Orbital states served from precise products
    pub precise_orbits: usize,

    /// Orbital states served from broadcast ephemerides
    pub broadcast_orbits: usize,
}

#[cfg(feature = "navigation")]
impl QcNavigationReport {
    /// New (empty) report for this receiver
    pub fn new(receiver: QcIndexing) -> Self {
        Self {
            receiver,
            epochs: 0,
            last_epoch: None,
            solutions: 0,
            failures: Default::default(),
            precise_orbits: 0,
            broadcast_orbits: 0,
        }
    }
}

/// CGGTTS track, resolved for one receiver
#[cfg(feature = "cggtts")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct QcCggttsTrack {
    /// Receiver
    pub receiver: QcIndexing,

    /// CGGTTS track (boxed: large compared to the other results)
    pub track: Box<cggtts::prelude::Track>,
}

/// CGGTTS summary, emitted once all signal sources ended
#[cfg(feature = "cggtts")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct QcCggttsReport {
    /// Receiver
    pub receiver: QcIndexing,

    /// Number of tracks released
    pub tracks: usize,

    /// Satellites tracked for less than the minimal track duration
    /// (rising, setting, data gaps): no track
    pub short_tracks: usize,

    /// Fit failures
    pub fit_errors: usize,
}

#[cfg(feature = "cggtts")]
impl QcCggttsReport {
    /// New (empty) report for this receiver
    pub fn new(receiver: QcIndexing) -> Self {
        Self {
            receiver,
            tracks: 0,
            short_tracks: 0,
            fit_errors: 0,
        }
    }
}

/// Ionospheric Pierce Point of one line of sight
#[cfg(feature = "ionex")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct QcIppOutput {
    /// Receiver
    pub receiver: QcIndexing,

    /// Satellite
    pub sv: SV,

    /// Orbital product the satellite position was resolved from
    pub orbit: crate::orbit::QcOrbitKind,

    /// Elevation of the line of sight, at the receiver (degrees)
    pub elevation_deg: f64,

    /// Azimuth of the line of sight, at the receiver (degrees)
    pub azimuth_deg: f64,

    /// Latitude of the pierce point (degrees)
    pub latitude_deg: f64,

    /// Longitude of the pierce point (degrees)
    pub longitude_deg: f64,

    /// Altitude of the ionospheric shell (km)
    pub altitude_km: f64,

    /// Obliquity (slant) factor at the pierce point: STEC = factor x VTEC
    pub obliquity_factor: f64,
}

/// Observation residual: A - B, for one satellite and one observable
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct QcObservationResidual {
    /// Receiver A (minuend)
    pub lhs: QcIndexing,

    /// Receiver B (subtrahend)
    pub rhs: QcIndexing,

    /// Satellite
    pub sv: SV,

    /// Observable (identical for both receivers)
    pub observable: rinex::prelude::Observable,

    /// A - B, in the unit of the observable: pseudo range (m), phase range (cycles),
    /// Doppler (Hz), signal strength (dB-Hz)
    pub residual: f64,

    /// Epoch of receiver B (the output epoch is the epoch of receiver A)
    pub rhs_epoch: Epoch,
}

/// Orbit residual: A - B between two orbital products, for one satellite
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct QcOrbitResidual {
    /// Satellite
    pub sv: SV,

    /// Source (agency) of product A
    pub lhs_source: QcIndexing,

    /// Kind of product A
    pub lhs_kind: crate::orbit::QcOrbitKind,

    /// Time of clock of the broadcast ephemeris (product A is broadcast)
    pub lhs_toc: Option<Epoch>,

    /// Source (agency) of product B
    pub rhs_source: QcIndexing,

    /// Kind of product B
    pub rhs_kind: crate::orbit::QcOrbitKind,

    /// Time of clock of the broadcast ephemeris (product B is broadcast)
    pub rhs_toc: Option<Epoch>,

    /// Position residual A - B, ECEF (m)
    pub ecef_m: (f64, f64, f64),

    /// Radial component (m)
    pub radial_m: f64,

    /// Along track component (m)
    pub along_track_m: f64,

    /// Cross track component (m)
    pub cross_track_m: f64,

    /// Clock residual A - B (s), when both products provide the clock
    pub clock_s: Option<f64>,
}

impl QcOrbitResidual {
    /// 3D position residual (m)
    pub fn norm_m(&self) -> f64 {
        let (x, y, z) = self.ecef_m;
        (x * x + y * y + z * z).sqrt()
    }
}

/// Results produced by the built-in processors.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum QcOutput {
    /// Signal observation (passthrough)
    Signal(QcSignalOutput),

    /// Ephemeris coverage change
    EphemerisCoverage(QcEphemerisCoverage),

    /// Radio ephemeris (passthrough)
    #[cfg_attr(feature = "serde", serde(skip_deserializing))]
    Ephemeris(QcEphemerisOutput),

    /// Observation residual
    ObservationResidual(QcObservationResidual),

    /// Orbit residual
    OrbitResidual(QcOrbitResidual),

    /// P.V.T solution
    #[cfg(feature = "navigation")]
    #[cfg_attr(feature = "serde", serde(skip_deserializing))]
    PVTSolution(QcPvtSolution),

    /// Navigation summary
    #[cfg(feature = "navigation")]
    NavigationReport(QcNavigationReport),

    /// Ionospheric Pierce Point
    #[cfg(feature = "ionex")]
    IPP(QcIppOutput),

    /// CGGTTS track
    #[cfg(feature = "cggtts")]
    CggttsTrack(QcCggttsTrack),

    /// CGGTTS summary
    #[cfg(feature = "cggtts")]
    CggttsReport(QcCggttsReport),
}

/// Time tagged result.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
