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

pub(crate) mod orbit_proj;
use orbit_proj::QcOrbitProjections;

mod sampling;

mod clock;
use clock::ClockResiduals;

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

    /// RTK Summary (if any)
    pub rtk_summary: Option<QcRTKSummary>,

    /// [QcContextSummary]
    pub summary: Option<QcContextSummary>,

    /// Pseudo range observations
    pub pseudo_range_observations: Option<QcObservationsReport>,
    
    /// Carrier Phase observations
    pub carrier_phase_observations: Option<QcObservationsReport>,
    
    /// Doppler observations
    pub doppler_observations: Option<QcObservationsReport>,

    /// Signal power observations
    pub signal_power_observations: Option<QcObservationsReport>,

    /// Geometry free combinations
    pub geometry_free_combinations: Option<QcObservationsReport>,

    /// Ionosphere free combinations
    pub ionosphere_free_combinations: Option<QcObservationsReport>,

    /// Melbourne Wubbena combinations
    pub melbourne_wubbena_combinations: Option<QcObservationsReport>,

    /// Carrier phase residuals
    pub carrier_phase_residuals: Option<QcObservationsReport>,

    /// Pseudo range residuals
    pub pseudo_range_residuals: Option<QcObservationsReport>,

    /// Multi path bias
    pub multipath_biases: Option<QcObservationsReport>,

    /// Sampling histogram
    pub sampling_histogram: Option<QcSamplingHistogram>,

    /// Possible SP3 Orbits proj
    #[cfg(all(feature = "navigation", feature = "sp3"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "navigation", feature = "sp3"))))]
    pub sp3_orbit_proj: Option<QcOrbitProjections>,

    /// Possible [QcNavReport]
    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    pub navi_report: Option<QcNavReport>,

    /// Clock residuals analysis report
    pub clock_residuals: Option<QcClockResiduals>,
}

impl QcRunReport {
    /// Initializes a [QcRunReport].
    pub(crate) fn new(deploy_time: Epoch, builder: &QcAnalysisBuilder) -> Self {
        let mut s = Self::default();
        s.run_summary.datetime = deploy_time;
        s
    }
}
