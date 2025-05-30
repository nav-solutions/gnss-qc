use crate::{
    context::{data::QcSourceDescriptor, QcProductType},
    error::QcError,
    processing::analysis::{QcAnalysis, QcAnalysisBuilder},
    report::{
        observations::QcObservationsReport, orbit_proj::QcOrbitProjections, rtk::QcRTKSummary,
        summaries::QcContextSummary, QcRunReport,
    },
    serializer::data::{QcSerializedItem, QcSerializedPreciseState},
};

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

    summary: bool,
    stores_signals: bool,
    rtk_summary: bool,

    #[cfg(feature = "navigation")]
    has_pvt_solver: bool,

    #[cfg(feature = "navigation")]
    stores_ephemeris: bool,

    /// [EphemerisBuffer]
    #[cfg(feature = "navigation")]
    ephemeris_buf: EphemerisBuffer,

    #[cfg(feature = "sp3")]
    precise_states_residuals: bool,

    #[cfg(feature = "sp3")]
    stores_precise_states: bool,

    /// [PreciseStateBuffer]
    #[cfg(feature = "sp3")]
    precise_states_buf: PreciseStateBuffer,
}

impl<'a> QcRunner<'a> {
    /// Deploy the [QcRunner]
    pub fn new(builder: &QcAnalysisBuilder, report: &'a mut QcRunReport) -> Result<Self, QcError> {
        let analysis = builder.build();

        let mut summary = false;
        let mut rtk_summary = false;
        let mut stores_signals = false;

        #[cfg(feature = "sp3")]
        let mut stores_precise_states = false;

        #[cfg(feature = "sp3")]
        let mut precise_states_residuals = false;

        #[cfg(feature = "navigation")]
        let mut stores_ephemeris = false;

        #[cfg(feature = "navigation")]
        let mut has_pvt_solver = false;

        for analysis in analysis.iter() {
            if matches!(
                analysis,
                QcAnalysis::SignalCombinations
                    | QcAnalysis::SignalObservations
                    | QcAnalysis::MultiPathBias
            ) {
                stores_signals = true;
            }

            if matches!(analysis, QcAnalysis::Summary) {
                summary = true;
            }

            if matches!(analysis, QcAnalysis::RTKSummary) {
                rtk_summary = true;
            }

            #[cfg(feature = "navigation")]
            if matches!(analysis, QcAnalysis::OrbitResiduals) {
                stores_ephemeris = true;
                precise_states_residuals = true;
            }

            #[cfg(feature = "navigation")]
            if matches!(analysis, QcAnalysis::ClockResiduals) {
                stores_ephemeris = true;
            }

            #[cfg(feature = "navigation")]
            if matches!(analysis, QcAnalysis::PVT(_) | QcAnalysis::CGGTTS(_)) {
                has_pvt_solver = true;
                stores_signals = true;
                stores_ephemeris = true;
                stores_precise_states = true;
            }
        }

        Ok(Self {
            summary,
            stores_signals,
            rtk_summary,

            #[cfg(feature = "navigation")]
            stores_ephemeris,

            #[cfg(feature = "navigation")]
            has_pvt_solver,

            #[cfg(feature = "navigation")]
            ephemeris_buf: EphemerisBuffer::new(),

            #[cfg(feature = "sp3")]
            stores_precise_states,

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
            QcSerializedItem::Ephemeris(item) =>
            {
                #[cfg(feature = "navigation")]
                if self.stores_ephemeris {
                    self.ephemeris_buf.latch(item);
                }
            }

            QcSerializedItem::RINEXHeader(item) => {
                if self.rtk_summary && item.product_type == QcProductType::Observation {
                    if let Some(summary) = &mut self.report.rtk_summary {
                        summary.latch_base_header(item.clone());
                    } else {
                        let mut summary = QcRTKSummary::default();
                        summary.latch_base_header(item.clone());
                        self.report.rtk_summary = Some(summary);
                    }
                }

                if self.summary {
                    let descriptor = QcSourceDescriptor {
                        indexing: item.indexing,
                        filename: item.filename,
                        product_type: item.product_type,
                    };

                    if let Some(summary) = &mut self.report.summary {
                        summary.latch_rinex(descriptor, item.data);
                    } else {
                        let mut summary = QcContextSummary::default();
                        summary.latch_rinex(descriptor, item.data);
                        self.report.summary = Some(summary);
                    }
                }
            }

            #[cfg(feature = "sp3")]
            QcSerializedItem::SP3Header(header) => {
                // latch new potential contribution
                if self.analysis.contains(&QcAnalysis::Summary) {
                    let descriptor = QcSourceDescriptor {
                        indexing: header.indexing,
                        filename: header.filename,
                        product_type: header.product_type,
                    };

                    if let Some(summary) = &mut self.report.summary {
                        summary.latch_sp3(descriptor, header.data);
                    } else {
                        let mut summary = QcContextSummary::default();
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
                if self.stores_signals {
                    if let Some(observations) = &mut self.report.observations {
                        observations.add_contribution(item);
                    } else {
                        let mut observation = QcObservationsReport::default();
                        observation.add_contribution(item);
                        self.report.observations = Some(observation);
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
