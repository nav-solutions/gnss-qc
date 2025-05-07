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

#[derive(Default, Debug, Clone)]
pub struct QcAnalysisBuilder {
    analysis: Vec<QcAnalysis>,
}

impl QcAnalysisBuilder {
    /// Execute all supported analysis (at the expense of more processing time)
    pub fn all(&self) -> Self {
        let s = Self::default()
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
        s.analysis.push(QcAnalysis::SP3Summary);
        s
    }

    #[cfg(feature = "sp3")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
    pub fn sp3_temporal_residuals(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::SP3Summary);
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

use crate::prelude::QcContext;
use crate::report::QcRunReport;
use crate::serializer::data::QcSerializedItem;

struct QcRunner {
    analysis: Vec<QcAnalysis>,
}

impl QcRunner {
    pub fn new(builder: QcAnalysisBuilder) -> Self {
        Self {
            analysis: builder.build(),
        }
    }

    pub fn consume(&mut self, sample: QcSerializedItem) {}
}

use crate::error::QcError;
use crate::prelude::Epoch;

impl QcContext {
    /// Process this [QcContext] running the following analysis specs.
    /// ## Input
    /// - current [QcContext]
    /// - analysis: [QcAnalysisBuilder] specs
    /// ## Output
    /// - synthesized: [QcRunReport] that you can then render in the desired format.
    pub fn process(&self, analysis: QcAnalysisBuilder) -> Result<QcRunReport, QcError> {
        let analysis = analysis.build();

        let deploy_time = Epoch::now().unwrap_or_else(|e| {
            panic!("Failed to determine system time: {}", e);
        });

        let mut report = QcRunReport::new(deploy_time, &analysis);
        let mut runner = QcRunner::new(analysis);

        let mut serializer = self.serializer();

        // pull data & consume
        while let Some(sample) = serializer.next() {
            runner.consume(sample);
        }

        Ok(report)
    }
}
