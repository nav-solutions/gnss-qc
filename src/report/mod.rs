//! Qc analysis report

mod sampling;

mod summary;
use summary::QcRunSummary;

#[cfg(doc)]
use crate::pipeline::QcPipeline;

/// [QcRunReport] is synthesized on [QcPipeline] completion.
/// It can then be rendered in several formats.
pub struct QcRunReport {
    /// [QcRunSummary]
    pub run_summary: QcRunSummary,
}
