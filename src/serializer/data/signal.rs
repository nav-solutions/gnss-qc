use rinex::{
    observation::LliFlags,
    prelude::Carrier,
    prelude::{Epoch, SV},
};

#[cfg(doc)]
use super::serializer::QcSerializer;

#[derive(Debug, Clone)]
pub enum QcSignalObservation {
    /// Pseudo Range in meters
    PseudoRange(f64),

    /// Phase Range in meters
    PhaseRange(f64),

    /// Doppler shift in Hz/s
    Doppler(f64),
}

/// [QcSignalData] streamed by [QcSerializer]
#[derive(Debug, Clone)]
pub struct QcSignalData {
    /// Sampling [Epoch]
    pub epoch: Epoch,

    /// [SV] signal source
    pub sv: SV,

    /// Possibly attached flags
    pub lli: Option<LliFlags>,

    /// [Carrier] frequency
    pub carrier: Carrier,

    /// [QcSignalObservation]
    pub observation: QcSignalObservation,
}
