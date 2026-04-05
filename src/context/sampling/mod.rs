//! Sampling characteristics and definitions
use crate::prelude::{ProductType, QcContext, TimeScale};

use qc_traits::{Merge, TimeCorrectionError, TimeCorrectionsDB, Timeshift};

/// [TemporalAxis] is associated to any sampled dataset,
/// for example GNSS observations, and is used to define the
/// temporal arc of an analysis.
pub struct TemporalAxis {
    /// Temporal axis is defined as a serie of [TimeSeries]
    /// to support a non-stable sampling rate.
    series: Vec<TimeSeries>,
}
