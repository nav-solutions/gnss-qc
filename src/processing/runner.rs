use crate::{
    context::data::QcSourceDescriptor,
    error::QcError,
    processing::analysis::{QcAnalysis, QcAnalysisBuilder},
    report::{summaries::QcContextSummary, QcRunReport},
    serializer::data::{QcSerializedEphemeris, QcSerializedItem},
};

pub struct QcRunner<'a> {
    /// List of [QcAnalysis]
    analysis: Vec<QcAnalysis>,

    /// [QcRunReport] being redacted
    report: &'a mut QcRunReport,

    /// Buffered Ephemeris
    ephemeris_buf: Vec<QcSerializedEphemeris>,
}

impl<'a> QcRunner<'a> {
    /// Deploy the [QcRunner]
    pub fn new(builder: &QcAnalysisBuilder, report: &'a mut QcRunReport) -> Result<Self, QcError> {
        Ok(Self {
            report,
            analysis: builder.build(),
            ephemeris_buf: Vec::with_capacity(32),
        })
    }

    pub fn stores_signals(&self) -> bool {
        if self.has_pvt_solver() {
            return true;
        }

        for analysis in self.analysis.iter() {
            if matches!(
                analysis,
                QcAnalysis::SignalCombinations | QcAnalysis::MultiPathBias
            ) {
                return true;
            }
        }

        self.analysis.contains(&QcAnalysis::SignalCombinations)
    }

    pub fn stores_ephemeris(&self) -> bool {
        if self.has_pvt_solver() {
            return true;
        }

        for analysis in self.analysis.iter() {
            if matches!(
                analysis,
                QcAnalysis::ClockResiduals | QcAnalysis::OrbitResiduals
            ) {
                return true;
            }
        }

        false
    }

    pub fn has_pvt_solver(&self) -> bool {
        for analysis in self.analysis.iter() {
            if matches!(analysis, QcAnalysis::PVT(_) | QcAnalysis::CGGTTS(_)) {
                return true;
            }
        }

        false
    }

    pub fn consume(&mut self, item: QcSerializedItem) {
        match item {
            QcSerializedItem::Ephemeris(ephemeris) => {
                if self.stores_ephemeris() {
                    self.ephemeris_buf.push(ephemeris);
                }
            }

            QcSerializedItem::RINEXHeader(header) => {
                // latch new potential contribution
                if self.analysis.contains(&QcAnalysis::Summary) {
                    let descriptor = QcSourceDescriptor {
                        indexing: header.indexing,
                        filename: header.filename,
                        product_type: header.product_type,
                    };

                    if let Some(summary) = &mut self.report.summary {
                        summary.latch_rinex(descriptor, header.data);
                    } else {
                        let mut summary = QcContextSummary::default();
                        summary.latch_rinex(descriptor, header.data);
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

            QcSerializedItem::Signal(signal) => {
                if self.analysis.contains(&QcAnalysis::SignalObservations) {
                    // TODO
                }
                if self.analysis.contains(&QcAnalysis::SignalCombinations) {
                    // TODO
                }
            }
        }
    }
}
