/// All supported [QcAnalysis]
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum QcAnalysis {
    Summary,
    RTKSummary,
    ClockResiduals,
    SignalObservations,
    MeteoObservations,

    ClockSummary,
    SignalCombinations,
    MultiPathBias,

    #[cfg(feature = "sp3")]
    SP3Summary,

    /// Broadcast versus Precise residual analysis.
    /// Wraps and summarizes each precise products
    /// and compares them to "real-time" navigation message gathered
    /// from Broadcast data.
    #[cfg(feature = "sp3")]
    OrbitResiduals,

    /// [QcAnalysis::NaviPlot] regroups projections and visualizations
    /// dedicated to fine analysis of the navigation conditions.
    /// This is the combination of the signal sampling conditions,
    /// skyview, navigation message, possible precise products
    /// and correction messages.
    #[cfg(feature = "navigation")]
    NaviPlot,

    #[cfg(feature = "sp3")]
    SP3TemporalResiduals,

    #[cfg(feature = "navigation")]
    PVT(PvtSolutions),

    #[cfg(all(feature = "navigation", feature = "cggtts"))]
    CGGTTS(PvtSolutions),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[cfg(feature = "navigation")]
pub enum PvtSolutions {
    /// Resolve all rovers to be identified
    PvtSolutionsAll,

    /// Resolve this rover uniquely
    PvtSolutionsSingleRover(String),
}

impl std::fmt::Display for QcAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Summary => write!(f, "Summary Report"),
            Self::RTKSummary => write!(f, "RTK Summary Report"),
            Self::NaviPlot => write!(f, "NAVI Plot"),
            Self::SignalCombinations => write!(f, "Signal Combinations"),
            Self::SignalObservations => write!(f, "Signal Observations"),
            Self::ClockResiduals => write!(f, "Clock Residuals"),
            Self::ClockSummary => write!(f, "Clock Summary"),
            Self::MeteoObservations => write!(f, "Meteo Observations"),
            Self::MultiPathBias => write!(f, "Multipath"),
            #[cfg(feature = "sp3")]
            Self::SP3TemporalResiduals => write!(f, "SP3 Clock Residuals"),
            #[cfg(feature = "sp3")]
            Self::SP3Summary => write!(f, "SP3 Summary"),
            #[cfg(feature = "sp3")]
            Self::OrbitResiduals => write!(f, "Orbital Residuals"),
            #[cfg(feature = "navigation")]
            Self::PVT(_) => write!(f, "P.V.T Solutions"),
            #[cfg(all(feature = "navigation", feature = "cggtts"))]
            Self::CGGTTS(_) => write!(f, "CGGTTS Solutions"),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct QcAnalysisBuilder {
    analysis: Vec<QcAnalysis>,
}

impl QcAnalysisBuilder {
    /// Perform all supported Analysis that the current Context may allow.
    pub fn all() -> Self {
        let s = Self::default()
            .summary_report()
            .summaries()
            .observations()
            .multipath_bias()
            .meteo_observations()
            .clock_residuals();

        #[cfg(feature = "sp3")]
        let s = s.sp3_summary();

        #[cfg(feature = "sp3")]
        let s = s.orbit_residuals();

        #[cfg(feature = "sp3")]
        let s = s.sp3_temporal_residuals();

        #[cfg(feature = "navigation")]
        let s = s.navi_plot();

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
        s.analysis.push(QcAnalysis::ClockSummary);
        s.analysis.push(QcAnalysis::RTKSummary);

        #[cfg(feature = "sp3")]
        s.analysis.push(QcAnalysis::SP3Summary);

        s
    }

    pub fn observations(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::SignalObservations);
        s
    }

    pub fn multipath_bias(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::MultiPathBias);
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

    /// Request to stack [QcAnalysis::NaviPlot] to the report to be redacted.
    #[cfg(feature = "navigation")]
    pub fn navi_plot(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::NaviPlot);
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
        s.analysis
            .push(QcAnalysis::PVT(PvtSolutions::PvtSolutionsAll));
        s
    }

    #[cfg(all(feature = "navigation", feature = "cggtts"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "navigation", feature = "cggtts"))))]
    pub fn nav_cggtts_solutions(&self) -> Self {
        let mut s = self.clone();
        s.analysis
            .push(QcAnalysis::CGGTTS(PvtSolutions::PvtSolutionsAll));
        s
    }
}
