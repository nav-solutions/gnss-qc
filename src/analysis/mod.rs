// Analysis we support
mod orbit_residual;
use orbit_residual::{OrbitPositionResidualDataPoint, OrbitVelocityResidualDataPoint};

mod temporal_residual;
use orbit_residual::{OrbitPositionResidualDataPoint, OrbitVelocityResidualDataPoint};

mod signal;
use signal::SignalObservationDataPoint;

mod meteo;
use meteo::MeteoSensorDataPoint;

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Analysis {
    ClockOffsetResidual,
    ClockDriftResidual,
    SignalObservation,
    MeteoSensorObservation,
    Sampling,
    ClockSummary,
    RoverSummary,
    BaseSummary,
    #[cfg(feature = "sp3")]
    SP3Summary,
    #[cfg(feature = "sp3")]
    OrbitPositionResidual,
    #[cfg(feature = "sp3")]
    OrbitVelocityResidual,
    #[cfg(feature = "navigation")]
    PVT,
    #[cfg(all(feature = "navigation", feature = "cggtts"))]
    CGGTTS,
}

pub enum DataPoint {
    SignalObservation(SignalObservationDataPoint),
    OrbitalPosition(OrbitalPositionDataPoint),
    ClockOffset(ClockOffsetDataPoint),
    ClockDrift(ClockDriftDataPoint),
    SignalObservation(SignalDataPoint),
    OrbitalPositionVelocity(OrbitalPositionVelocityDataPoint),
    MeteoSensorObservation(MeteoSensorDataPoint),
    #[cfg(feature = "sp3")]
    HighPrecisionOrbitalPosition(OrbitalPositionDataPoint),
    #[cfg(feature = "sp3")]
    HighPrecisionOrbitalPositionVelocity(OrbitalPositionDataPoint),
}
