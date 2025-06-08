use crate::{
    context::{data::QcSourceDescriptor, QcProductType},
    error::QcError,
    processing::analysis::{QcAnalysis, QcAnalysisBuilder},
    report::{
        nav::QcNavReport, observations::QcObservationsReport, orbit_proj::QcOrbitProjections,
        rtk::QcRTKSummary, summaries::QcContextSummary, QcRunReport,
    },
    serializer::data::{QcSerializedItem, QcSerializedPreciseState},
};

mod signals_buf;
mod temporal_data;

use signals_buf::SignalsBuffer;

#[cfg(feature = "navigation")]
use crate::prelude::Frame;

#[cfg(feature = "navigation")]
mod ephemeris_buf;

#[cfg(feature = "navigation")]
use ephemeris_buf::EphemerisBuffer;

#[cfg(feature = "sp3")]
mod precise_states;

#[cfg(feature = "sp3")]
use precise_states::PreciseStateBuffer;

pub struct QcRunner<'a> {
    /// List of [QcAnalysis]
    analysis: Vec<QcAnalysis>,

    /// [QcRunReport] being redacted
    report: &'a mut QcRunReport,

    /// Reference [Frame]
    #[cfg(feature = "navigation")]
    frame: Frame,

    rinex_summary: bool,
    phase_observations: bool,
    doppler_observations: bool,
    power_observations: bool,
    pseudo_range_observations: bool,

    rtk_summary: bool,

    #[cfg(feature = "navigation")]
    navi_plot: bool,

    #[cfg(feature = "navigation")]
    has_pvt_solver: bool,

    #[cfg(feature = "navigation")]
    stores_ephemeris: bool,

    /// [EphemerisBuffer]
    #[cfg(feature = "navigation")]
    ephemeris_buf: EphemerisBuffer<'a>,

    /// [SignalsBuffer]
    signals_buf: SignalsBuffer,

    #[cfg(feature = "sp3")]
    sp3_summary: bool,

    #[cfg(feature = "sp3")]
    precise_states_residuals: bool,

    #[cfg(feature = "sp3")]
    stores_precise_states: bool,

    /// [PreciseStateBuffer]
    #[cfg(feature = "sp3")]
    precise_states_buf: PreciseStateBuffer<'a>,

    #[cfg(all(feature = "navigation", feature = "sp3"))]
    orbit_residuals: bool,
}

impl<'a> QcRunner<'a> {
    /// Deploy the [QcRunner]
    pub fn new(
        builder: &QcAnalysisBuilder,
        report: &'a mut QcRunReport,
        frame: Frame,
    ) -> Result<Self, QcError> {
        let analysis = builder.build();

        let mut rinex_summary = false;
        let mut rtk_summary = false;

        let mut power_observations = false;
        let mut phase_observations = false;
        let mut pseudo_range_observations = false;
        let mut doppler_observations = false;

        #[cfg(feature = "sp3")]
        let mut sp3_summary = false;

        #[cfg(feature = "sp3")]
        let mut stores_precise_states = false;

        #[cfg(feature = "sp3")]
        let mut precise_states_residuals = false;

        #[cfg(feature = "navigation")]
        let mut stores_ephemeris = false;

        #[cfg(feature = "navigation")]
        let mut navi_plot = false;

        #[cfg(feature = "navigation")]
        let mut orbit_residuals = false;

        #[cfg(feature = "navigation")]
        let mut has_pvt_solver = false;

        for analysis in analysis.iter() {
            match analysis {
                QcAnalysis::DopplerObservations => {
                    doppler_observations = true;
                }
                QcAnalysis::PseudoRangeObservations => {
                    pseudo_range_observations = true;
                }
                QcAnalysis::PhaseObservations => {
                    phase_observations = true;
                }
                QcAnalysis::SignalPowerObservations => {
                    power_observations = true;
                }
                QcAnalysis::RINEXSummary => {
                    rinex_summary = true;
                }
                _ => {}
            }

            #[cfg(feature = "navigation")]
            match analysis {
                QcAnalysis::RTKSummary => {
                    rtk_summary = true;
                }
                QcAnalysis::NaviPlot => {
                    navi_plot = true;
                    stores_ephemeris = true;
                }
                QcAnalysis::PVT(_) | QcAnalysis::CGGTTS(_) => {
                    stores_ephemeris = true;
                }
                _ => {}
            }

            #[cfg(feature = "sp3")]
            match analysis {
                QcAnalysis::SP3Summary => {
                    sp3_summary = true;
                }
                QcAnalysis::OrbitResiduals => {
                    orbit_residuals = true;
                    stores_ephemeris = true;
                }
                QcAnalysis::ClockResiduals => {
                    stores_ephemeris = true;
                }
                _ => {}
            }
        }

        Ok(Self {
            rinex_summary,
            rtk_summary,
            phase_observations,
            doppler_observations,
            power_observations,
            pseudo_range_observations,

            signals_buf: SignalsBuffer::new(),

            #[cfg(feature = "navigation")]
            frame,

            #[cfg(feature = "navigation")]
            navi_plot,

            #[cfg(all(feature = "navigation", feature = "sp3"))]
            orbit_residuals,

            #[cfg(feature = "navigation")]
            stores_ephemeris,

            #[cfg(feature = "navigation")]
            has_pvt_solver,

            #[cfg(feature = "navigation")]
            ephemeris_buf: EphemerisBuffer::new(),

            #[cfg(feature = "sp3")]
            stores_precise_states,

            #[cfg(feature = "sp3")]
            sp3_summary,

            #[cfg(feature = "sp3")]
            precise_states_residuals,

            #[cfg(feature = "sp3")]
            precise_states_buf: PreciseStateBuffer::new(),

            report,
            analysis,
        })
    }

    pub fn consume(&mut self, item: QcSerializedItem) {
        match item {
            QcSerializedItem::Ephemeris(item) => {
                #[cfg(feature = "navigation")]
                if self.stores_ephemeris {
                    self.ephemeris_buf.latch(item);
                }

                #[cfg(feature = "navigation")]
                if self.navi_plot {
                    if let Some(report) = &mut self.report.navi_report {
                        report.add_ephemeris_message(&item);
                    } else {
                        let mut report = QcNavReport::default();
                        report.add_ephemeris_message(&item);
                        self.report.navi_report = Some(report);
                    }
                }
            }

            QcSerializedItem::RINEXHeader(item) => {
                if self.rtk_summary && item.product_type == QcProductType::Observation {
                    if let Some(summary) = &mut self.report.rtk_summary {
                        summary.latch_base_header(item);
                    } else {
                        let mut summary = QcRTKSummary::new(self.frame);
                        summary.latch_base_header(item);
                        self.report.rtk_summary = Some(summary);
                    }
                }

                if self.rinex_summary {
                    let descriptor = QcSourceDescriptor {
                        indexing: item.indexing,
                        filename: item.filename,
                        product_type: item.product_type,
                    };

                    if let Some(summary) = &mut self.report.summary {
                        summary.latch_rinex(descriptor, item.data);
                    } else {
                        let mut summary = QcContextSummary::new(self.frame);
                        summary.latch_rinex(descriptor, item.data);
                        self.report.summary = Some(summary);
                    }
                }
            }

            #[cfg(feature = "sp3")]
            QcSerializedItem::SP3Header(header) => {
                // latch new potential contribution
                if self.analysis.contains(&QcAnalysis::SP3Summary) {
                    let descriptor = QcSourceDescriptor {
                        indexing: header.indexing,
                        filename: header.filename,
                        product_type: header.product_type,
                    };

                    if let Some(summary) = &mut self.report.summary {
                        summary.latch_sp3(descriptor, header.data);
                    } else {
                        let mut summary = QcContextSummary::new(self.frame);
                        summary.latch_sp3(descriptor, header.data);
                        self.report.summary = Some(summary);
                    }
                }
            }

            #[cfg(feature = "sp3")]
            QcSerializedItem::PreciseState(item) => {
                if self.precise_states_residuals {
                    self.run_precise_states_residuals(item);
                }
            }

            QcSerializedItem::Signal(item) => {
                if self.doppler_observations
                    || self.phase_observations
                    || self.doppler_observations
                    || self.power_observations
                {
                    if let Some(report) = &mut self.report.observations {
                        report.latch_signal(&item);
                    } else {
                        let mut report = QcObservationsReport::new(
                            self.phase_observations,
                            self.doppler_observations,
                            self.pseudo_range_observations,
                            self.power_observations,
                        );
                        report.latch_signal(&item);
                        self.report.observations = Some(report);
                    }
                }

                #[cfg(feature = "navigation")]
                if self.navi_plot {
                    if let Some(report) = &mut self.report.navi_report {
                        report.add_signal_contribution(&item);
                    } else {
                        let mut report = QcNavReport::default();
                        report.add_signal_contribution(&item);
                        self.report.navi_report = Some(report);
                    }
                }

                #[cfg(feature = "navigation")]
                if self.stores_ephemeris {
                    // self.ephemeris_buf.update(item.data.epoch);
                }
            }
        }
    }

    fn run_precise_states_residuals(&mut self, item: QcSerializedPreciseState) {
        if let Some(orbit_projs) = &mut self.report.sp3_orbits_proj {
            orbit_projs.latch_precise_state(item);
        } else {
            let proj = QcOrbitProjections::from_precise_state(item);
            self.report.sp3_orbits_proj = Some(proj);
        }
    }
}
