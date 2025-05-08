//! Qc analysis report
pub mod sampling;

use crate::{prelude::Epoch, processing::analysis::QcAnalysisBuilder};

mod run_summary;
use run_summary::QcRunSummary;

pub(crate) mod summaries;
use summaries::QcContextSummary;

mod observations;
use observations::QcObservationsReport;

pub mod temporal_data;

pub(crate) mod orbit_proj;

use orbit_proj::QcOrbitProjections;

#[cfg(feature = "html")]
#[cfg_attr(docsrs, doc(cfg(feature = "html")))]
mod html;

// #[cfg(doc)]
// use crate::pipeline::QcPipeline;

/// [QcRunReport] is synthesized on [QcPipeline] completion.
/// It can then be rendered in several formats.
#[derive(Clone, Default)]
pub struct QcRunReport {
    /// [QcRunSummary]
    pub run_summary: QcRunSummary,

    /// [QcContextSummary]
    pub summary: Option<QcContextSummary>,

    /// Reported observations
    pub observations: Option<QcObservationsReport>,

    /// Possible SP3 Orbits proj
    #[cfg_attr(docsrs, doc(cfg(all(feature = "navigation", feature = "sp3"))))]
    pub sp3_orbits_proj: Option<QcOrbitProjections>,
}

impl QcRunReport {
    pub(crate) fn new(deploy_time: Epoch, analysis: &QcAnalysisBuilder) -> Self {
        let mut s = Self::default();

        s.run_summary.datetime = deploy_time;
        s.run_summary.analysis = analysis.build();

        s
    }
}
