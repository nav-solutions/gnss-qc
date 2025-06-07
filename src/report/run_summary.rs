use crate::{
    prelude::{Duration, Epoch},
    processing::analysis::QcAnalysis,
};

/// [QcPipeline] run summary.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct QcRunSummary {
    /// Deployment datetime, as [Epoch]
    pub datetime: Epoch,

    /// Total processing time, as [Duration]
    pub run_duration: Duration,

    /// Analysis that were selected
    pub analysis: Vec<QcAnalysis>,
}
