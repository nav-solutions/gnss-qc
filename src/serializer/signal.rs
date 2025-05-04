use rinex::prelude::Carrier;

use crate::prelude::SV;

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

/// [QcSignalDataPoint] streamed by [QcSerializer]
#[derive(Debug, Clone)]
pub struct QcSignalDataPoint {
    /// Readable name of origin
    pub source_name: String,

    /// [SV] signal source
    pub sv: SV,

    /// [Carrier] frequency
    pub carrier: Carrier,

    /// [QcSignalObservation]
    pub observation: QcSignalObservation,
}
