//! Built-in processors.
//!
//! Every processor implements [QcProcessor](crate::engine::QcProcessor) for any output
//! type `O: From<QcOutput>`, so they can be stacked next to application specific processors.
mod coverage;
mod ephemeris;
mod observation_residuals;
mod orbit_residuals;
mod signal;

#[cfg(feature = "ionex")]
mod ipp;

pub use coverage::QcEphemerisCoverageProcessor;
pub use ephemeris::QcEphemerisProcessor;
pub use observation_residuals::{QcObservationResidualConfig, QcObservationResidualProcessor};
pub use orbit_residuals::{QcOrbitProduct, QcOrbitResidualConfig, QcOrbitResidualProcessor};
pub use signal::QcSignalProcessor;

#[cfg(feature = "ionex")]
pub use ipp::{elevation_azimuth_deg, QcIppConfig, QcIppProcessor};
