//! Qc analysis report
pub mod sampling;

use crate::{analysis::QcAnalysisBuilder, prelude::Epoch};

mod summary;
use summary::QcRunSummary;

pub(crate) mod ctx_summary;
use ctx_summary::QcContextSummary;

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

    /// [QcContextSummary]
    pub ctx_summary: Option<QcContextSummary>,

    /// Reported observations
    pub observations: Option<QcObservationsReport>,
}

impl QcRunReport {
    pub(crate) fn new(deploy_time: Epoch, analysis: &QcAnalysisBuilder) -> Self {
        let mut s = Self::default();

        s.run_summary.datetime = deploy_time;
        s.run_summary.analysis = analysis.build();

        s
    }
}
