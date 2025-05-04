use crate::{
    pipeline::job::QcJob,
    prelude::{Duration, Epoch},
};

/// [QcPipeline] run summary.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct QcRunSummary {
    /// Deployment datetime as [Epoch]
    pub datetime: Epoch,

    /// Processing [Duration]
    pub run_duration: Duration,

    /// Number of executes jobs
    pub num_jobs: usize,

    /// [QcJob]s that were executed
    pub jobs: Vec<QcJob>,
}
