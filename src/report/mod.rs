//! Qc analysis report

pub mod sampling;

pub mod summary;
use summary::QcRunSummary;

pub mod observations;

pub mod temporal_data;

#[cfg(doc)]
use crate::pipeline::QcPipeline;

/// [QcRunReport] is synthesized on [QcPipeline] completion.
/// It can then be rendered in several formats.
pub struct QcRunReport {
    /// [QcRunSummary]
    pub run_summary: QcRunSummary,
}
