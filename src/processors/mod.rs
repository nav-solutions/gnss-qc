//! Built-in processors.
//!
//! Every processor implements [QcProcessor](crate::engine::QcProcessor) for any output
//! type `O: From<QcOutput>`, so they can be stacked next to application specific processors.
mod coverage;
mod signal;

pub use coverage::QcEphemerisCoverageProcessor;
pub use signal::QcSignalProcessor;
