//! Qc analysis report
use crate::{prelude::Epoch, processing::analysis::QcAnalysisBuilder};

mod run_summary;
use run_summary::QcRunSummary;

pub(crate) mod summaries;
use summaries::QcContextSummary;

pub(crate) mod observations;
use observations::QcObservationsReport;

pub(crate) mod rtk;
use rtk::QcRTKSummary;

pub mod temporal_data;

pub(crate) mod orbit_proj;

use orbit_proj::QcOrbitProjections;

#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
pub mod nav;

#[cfg(feature = "navigation")]
use nav::QcNavReport;

pub mod rendering;

/// [QcRunReport] is synthesized on [QcPipeline] completion.
/// It can then be rendered in several formats.
#[derive(Default)]
pub struct QcRunReport {
    /// [QcRunSummary]
    pub run_summary: QcRunSummary,

    /// [QcContextSummary]
    pub summary: Option<QcContextSummary>,

    /// Reported observations
    pub observations: Option<QcObservationsReport>,

    /// RTK Summary (if any)
    pub rtk_summary: Option<QcRTKSummary>,

    /// Possible SP3 Orbits proj
    #[cfg(all(feature = "navigation", feature = "sp3"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "navigation", feature = "sp3"))))]
    pub sp3_orbits_proj: Option<QcOrbitProjections>,

    /// Possible [QcNavReport]
    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    pub navi_report: Option<QcNavReport>,
}

impl QcRunReport {
    /// Initializes a [QcRunReport].
    pub(crate) fn new(deploy_time: Epoch, analysis: &QcAnalysisBuilder) -> Self {
        let mut s = Self::default();

        s.run_summary.datetime = deploy_time;
        s.run_summary.analysis = analysis.build();

        s
    }
}
