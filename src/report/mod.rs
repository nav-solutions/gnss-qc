//! Qc analysis report
pub mod sampling;
use crate::prelude::Epoch;

mod summary;

use summary::QcRunSummary;

mod observations;
use observations::QcObservationsReport;

pub mod temporal_data;

#[cfg(feature = "html")]
#[cfg_attr(docsrs, doc(cfg(feature = "html")))]
mod html;

// #[cfg(doc)]
// use crate::pipeline::QcPipeline;

/// [QcRunReport] is synthesized on [QcPipeline] completion.
/// It can then be rendered in several formats.
#[derive(Clone, Debug, Default)]
pub struct QcRunReport {
    /// [QcRunSummary]
    pub run_summary: QcRunSummary,

    /// Reported observations
    pub observations: Option<QcObservationsReport>,
}

impl QcRunReport {
    pub(crate) fn new(deploy_time: Epoch, num_jobs: usize) -> Self {
        let mut s = Self::default();

        s.run_summary.datetime = deploy_time;
        s.run_summary.num_jobs = num_jobs;

        s
    }
}
