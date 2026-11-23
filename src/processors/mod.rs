//! Built-in processors.
//!
//! Every processor implements [QcProcessor](crate::engine::QcProcessor) for any output
//! type `O: From<QcOutput>`, so they can be stacked next to application specific processors.
mod coverage;
mod ephemeris;
mod signal;

#[cfg(feature = "ionex")]
mod ipp;

pub use coverage::QcEphemerisCoverageProcessor;
pub use ephemeris::QcEphemerisProcessor;
pub use signal::QcSignalProcessor;

#[cfg(feature = "ionex")]
pub use ipp::{elevation_azimuth_deg, QcIppConfig, QcIppProcessor};
