//! Built-in processors.
//!
//! Every processor implements [QcProcessor](crate::engine::QcProcessor) for any output
//! type `O: From<QcOutput>`, so they can be stacked next to application specific processors.
mod clock;
mod clock_residuals;
mod coverage;
mod ephemeris;
mod observation_residuals;
mod orbit_residuals;
mod signal;
mod tec;

#[cfg(feature = "ionex")]
mod ipp;

pub use clock::{QcMeteoConfig, QcMeteoProcessor, QcPreciseClockConfig, QcPreciseClockProcessor};
pub use clock_residuals::{
    QcClockAlignment, QcClockProduct, QcClockResidualConfig, QcClockResidualProcessor,
};
pub use coverage::QcEphemerisCoverageProcessor;
pub use ephemeris::QcEphemerisProcessor;
pub use observation_residuals::{QcObservationResidualConfig, QcObservationResidualProcessor};
pub use orbit_residuals::{QcOrbitProduct, QcOrbitResidualConfig, QcOrbitResidualProcessor};
pub use signal::QcSignalProcessor;
pub use tec::{QcTecConfig, QcTecProcessor, QcTecRegion, QcTecSampling};

#[cfg(feature = "ionex")]
pub use ipp::{elevation_azimuth_deg, QcIppConfig, QcIppProcessor};
