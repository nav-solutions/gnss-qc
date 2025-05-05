//! Qc analysis report

pub mod sampling;

mod summary;
use summary::QcRunSummary;

mod observations;
use observations::QcObservationsReport;

use crate::serializer::data::QcSerializedSignal;

pub mod temporal_data;

// #[cfg(doc)]
// use crate::pipeline::QcPipeline;

/// [QcRunReport] is synthesized on [QcPipeline] completion.
/// It can then be rendered in several formats.
#[derive(Clone, Debug, Default)]
pub struct QcRunReport {
    /// [QcRunSummary]
    pub run_summary: QcRunSummary,

    /// Reported observations
    pub observations: QcObservationsReport,
}

impl QcRunReport {
    /// Add new report contribution
    pub fn add_signal_contribution(&mut self, data: QcSerializedSignal) {
        // Contributes to observations (obviously)
        self.observations.add_contribution(data);
    }
}
