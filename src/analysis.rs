/// All supported [QcAnalysis]
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub(crate) enum QcAnalysis {
    Summary,
    ClockResiduals,
    SignalObservations,
    MeteoObservations,
    Sampling,
    ClockSummary,
    RoverSummary,
    BaseSummary,
    SignalCombinations,
    MultiPathBias,

    #[cfg(feature = "sp3")]
    SP3Summary,

    #[cfg(feature = "sp3")]
    OrbitResiduals,

    #[cfg(feature = "sp3")]
    SP3TemporalResiduals,

    #[cfg(feature = "navigation")]
    PVT,

    #[cfg(all(feature = "navigation", feature = "cggtts"))]
    CGGTTS,
}

impl std::fmt::Display for QcAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sampling => write!(f, "Sampling Analysis"),
            Self::SignalCombinations => write!(f, "Signal Combinations"),
            Self::SignalObservations => write!(f, "Signal Observations"),
            Self::BaseSummary => write!(f, "RTK Base(s) summary"),
            Self::ClockResiduals => write!(f, "Clock Residuals"),
            Self::ClockSummary => write!(f, "Clock Summary"),
            Self::MeteoObservations => write!(f, "Meteo Observations"),
            Self::MultiPathBias => write!(f, "Multipath"),
            Self::RoverSummary => write!(f, "Rover(s) summary"),
            Self::Summary => write!(f, "Summary report"),
            #[cfg(feature = "sp3")]
            Self::SP3TemporalResiduals => write!(f, "SP3 Clock Residuals"),
            #[cfg(feature = "sp3")]
            Self::SP3Summary => write!(f, "SP3 Summary"),
            #[cfg(feature = "sp3")]
            Self::OrbitResiduals => write!(f, "Orbital Residuals"),
            #[cfg(feature = "navigation")]
            Self::PVT => write!(f, "P.V.T Solutions"),
            #[cfg(all(feature = "navigation", feature = "cggtts"))]
            Self::CGGTTS => write!(f, "CGGTTS Solutions"),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct QcAnalysisBuilder {
    analysis: Vec<QcAnalysis>,
}

impl QcAnalysisBuilder {
    /// Execute all supported analysis (at the expense of more processing time)
    pub fn all() -> Self {
        let s = Self::default()
            .summary_report()
            .summaries()
            .sampling()
            .observations()
            .meteo_observations()
            .clock_residuals();

        #[cfg(feature = "sp3")]
        let s = s.sp3_summary();

        #[cfg(feature = "sp3")]
        let s = s.orbit_residuals();

        #[cfg(feature = "sp3")]
        let s = s.sp3_temporal_residuals();

        #[cfg(feature = "navigation")]
        let s = s.nav_pvt_solutions();

        #[cfg(all(feature = "navigation", feature = "cggtts"))]
        let s = s.nav_cggtts_solutions();

        s
    }

    pub(crate) fn build(&self) -> Vec<QcAnalysis> {
        self.analysis.clone()
    }

    /// The summary report will report input products that were encountered.
    pub fn summary_report(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::Summary);
        s
    }

    /// Activate summary reports of all supported types
    pub fn summaries(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::RoverSummary);
        s.analysis.push(QcAnalysis::ClockSummary);
        s.analysis.push(QcAnalysis::BaseSummary);

        #[cfg(feature = "sp3")]
        s.analysis.push(QcAnalysis::SP3Summary);

        s
    }

    pub fn sampling(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::Sampling);
        s
    }

    pub fn observations(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::SignalObservations);
        s
    }

    pub fn meteo_observations(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::MeteoObservations);
        s
    }

    pub fn clock_residuals(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::ClockResiduals);
        s
    }

    #[cfg(feature = "sp3")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
    pub fn sp3_summary(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::SP3Summary);
        s
    }

    #[cfg(feature = "sp3")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
    pub fn orbit_residuals(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::OrbitResiduals);
        s
    }

    #[cfg(feature = "sp3")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
    pub fn sp3_temporal_residuals(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::SP3TemporalResiduals);
        s
    }

    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    pub fn nav_pvt_solutions(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::PVT);
        s
    }

    #[cfg(all(feature = "navigation", feature = "cggtts"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "navigation", feature = "cggtts"))))]
    pub fn nav_cggtts_solutions(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::CGGTTS);
        s
    }
}

use std::collections::HashMap;

use crate::context::data::QcSourceDescriptor;
use crate::prelude::QcContext;

use crate::report::{ctx_summary::QcContextSummary, QcRunReport};

use crate::serializer::data::{QcSerializedEphemeris, QcSerializedItem};

struct QcRunner<'a> {
    /// List of analysis
    analysis: Vec<QcAnalysis>,

    /// Report being redacted
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

    pub fn stores_ephemeris(&self) -> bool {
        for analysis in self.analysis.iter() {
            if matches!(
                analysis,
                QcAnalysis::ClockResiduals
                    | QcAnalysis::CGGTTS
                    | QcAnalysis::OrbitResiduals
                    | QcAnalysis::PVT
            ) {
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

                    if let Some(summary) = &mut self.report.ctx_summary {
                        summary.latch_rinex(descriptor, header.data);
                    } else {
                        let mut summary = QcContextSummary::default();
                        summary.latch_rinex(descriptor, header.data);
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

                    if let Some(summary) = &mut self.report.ctx_summary {
                        summary.latch_sp3(descriptor, header.data);
                    } else {
                        let mut summary = QcContextSummary::default();
                        summary.latch_sp3(descriptor, header.data);
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

use crate::error::QcError;

use hifitime::prelude::{Epoch, Unit};

impl QcContext {
    /// Process this [QcContext] running the following analysis specs.
    /// ## Input
    /// - current [QcContext]
    /// - analysis: [QcAnalysisBuilder] specs
    /// ## Output
    /// - synthesized: [QcRunReport] that you can then render in the desired format.
    pub fn process(&self, analysis: QcAnalysisBuilder) -> Result<QcRunReport, QcError> {
        let mut serializer = self.serializer();

        let deploy_time = Epoch::now()
            .map_err(|e| {
                error!("Failed to determine system time: {}", e);
                QcError::SystemTimeError
            })?
            .round(1.0 * Unit::Second);

        info!("process starting: {}", deploy_time);

        let mut report = QcRunReport::new(deploy_time, &analysis);

        let mut runner = QcRunner::new(&analysis, &mut report)?;

        // pull & consume data
        while let Some(sample) = serializer.next() {
            runner.consume(sample);
        }

        let end_time = Epoch::now()
            .unwrap_or_else(|e| {
                panic!("Failed to determine system time: {}", e);
            })
            .round(1.0 * Unit::Second);

        let run_duration = end_time - deploy_time;
        report.run_summary.run_duration = run_duration;

        info!("process concluded: {}", end_time);
        debug!("run duration: {}", run_duration);

        Ok(report)
    }
}

#[cfg(test)]
mod test {

    use std::fs::File;
    use std::io::Write;

    use crate::{prelude::QcContext, tests::init_logger};

    use super::QcAnalysisBuilder;

    #[test]
    fn process_full_run() {
        init_logger();

        let mut ctx = QcContext::new();

        // load data
        ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
            .unwrap();

        ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
            .unwrap();

        ctx.load_gzip_rinex_file("data/CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz")
            .unwrap();

        ctx.load_gzip_sp3_file("data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz")
            .unwrap();

        let builder = QcAnalysisBuilder::all();

        let report = ctx.process(builder).unwrap();

        let html = report.render_html().into_string();
        let mut fd = File::create("index.html").unwrap();
        write!(fd, "{}", html).unwrap();
    }
}
