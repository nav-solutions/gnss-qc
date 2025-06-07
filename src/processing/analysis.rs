/// All supported [QcAnalysis]
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum QcAnalysis {
    /// [QcAnalysis::Summary] will generate a summary table
    /// (as an introduction) for all RINEX products.
    /// This gives high level information such as timescale being used
    /// and information about the data production setup.
    Summary,

    /// [QcAnalysis::SignalObservations] will generate a cartesian 2D
    /// projection of each observation per data source, per individual Constellation.
    SignalObservations,

    /// [QcAnalysis::MeteoObservations] will generate a cartesian 2D projection
    /// of meteo sensors, in case such data was provided.
    MeteoObservations,

    /// [QcAnalaysis::RTKSummary] is meaningful as long as you
    /// loaded two different RINEX sources. The ROVER/Base definitions
    /// do not have to be complete, because this library considers
    /// data-sources as Base by default. [QcAnalysis::RTKSummary]
    /// gives you a better understanding and projection of the RTK regional area.
    RTKSummary,

    ClockSummary,

    SignalCombinations,
    MultiPathBias,

    /// [QcAnalysis::ClockResiduals] will evaluate and render (as cartesian
    /// 2D projections) the residual error between the clock states resolved
    /// from Broadcast radio message, and post-processed products (in case
    /// such have been loaded).
    ClockResiduals,

    /// [QcAnalysis::SP3Summary] will generate a summary table
    /// for each SP3 product, giving high level information like
    /// data producer, reference frame and coordinates system, or timescale
    /// being used.
    #[cfg(feature = "sp3")]
    SP3Summary,

    #[cfg(feature = "sp3")]
    /// [QcAnalysis::OrbitResiduals] will evaluate and render (as cartesian
    /// 2D/3D projections) the residual error between the orbital states resolved
    /// from Broadcast radio message, and post-processed products (in case
    /// such have been loaded).
    OrbitResiduals,

    /// [QcAnalysis::NaviPlot] regroups projections and visualizations
    /// dedicated to fine analysis of the navigation conditions.
    /// This is the combination of the signal sampling conditions,
    /// skyview, navigation message, possible precise products
    /// and correction messages.
    #[cfg(feature = "navigation")]
    NaviPlot,

    #[cfg(feature = "sp3")]
    /// [QcAnalysis::SP3TemporalResiduals] will evaluate and render (as cartesian
    /// 2D projections) the residual error between the clock states resolved
    /// from Broadcast radio message, and the post-processed clock state obtained
    /// from SP3 (in case such have been loaded).
    SP3TemporalResiduals,

    #[cfg(feature = "navigation")]
    /// [QcAnalysis::PVT] will attach [PvtSolutions] to this report being synthesized.
    /// [PvtSolutions] regroups several projections and a lot of information,
    /// about the receiver, receiving conditions and local environment.
    PVT(PvtSolutions),

    #[cfg(all(feature = "navigation", feature = "cggtts"))]
    /// [QcAnalysis::CGGTTS] will run the special CGGTTS post-fit over the
    /// [PvtSolutions]. This is dedicated to the local clock state,
    /// then resolved with higher precision, and typically used in remote
    /// (post-processed) clock synchronization.
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

    /// Generate a summary table
    /// (as an introduction) for all RINEX products.
    /// This gives high level information such as timescale being used
    /// and information about the data production setup.
    pub fn rinex_summary(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::Summary);
        s
    }

    /// Generate a summary table
    /// (as an introduction) for all RINEX and SP3 products.
    /// This gives high level information such as timescale being used
    /// and information about the data production setup.
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

    /// Generate a cartesian 2D projection
    /// of meteo sensor measurements, in case such data was provided.
    pub fn meteo_observations(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::MeteoObservations);
        s
    }

    /// Evaluate and render (as cartesian
    /// 2D projections) the residual error between the clock states resolved
    /// from Broadcast radio message, and post-processed products (in case
    /// such have been loaded).
    pub fn clock_residuals(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::ClockResiduals);
        s
    }

    #[cfg(feature = "navigation")]
    /// Regroups projections and visualizations
    /// dedicated to in-depth analysis of the navigation conditions.
    /// This is the combination of the signal sampling conditions,
    /// skyview, navigation message, possible precise products
    /// and correction messages.
    pub fn navi_plot(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::NaviPlot);
        s
    }

    #[cfg(feature = "sp3")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
    /// Generate a summary table
    /// for each SP3 product, giving high level information like
    /// data producer, reference frame and coordinates system, or timescale
    /// being used.
    pub fn sp3_summary(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::SP3Summary);
        s
    }

    #[cfg(feature = "sp3")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
    /// Evaluate and render (as cartesian
    /// 2D/3D projections) the residual error between the orbital states resolved
    /// from Broadcast radio message, and post-processed products (in case
    /// such have been loaded).
    pub fn orbit_residuals(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::OrbitResiduals);
        s
    }

    #[cfg(feature = "sp3")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
    /// Evaluate and render (as cartesian
    /// 2D projections) the residual error between the clock states resolved
    /// from Broadcast radio message, and the post-processed clock state obtained
    /// from SP3 (in case such have been loaded).
    pub fn sp3_temporal_residuals(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::SP3TemporalResiduals);
        s
    }

    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    /// Attach [PvtSolutions] to this report being synthesized.
    /// [PvtSolutions] regroups several projections and a lot of information,
    /// about the receiver, receiving conditions and local environment.
    pub fn nav_pvt_solutions(&self) -> Self {
        let mut s = self.clone();
        s.analysis
            .push(QcAnalysis::PVT(PvtSolutions::PvtSolutionsAll));
        s
    }

    #[cfg(all(feature = "navigation", feature = "cggtts"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "navigation", feature = "cggtts"))))]
    /// Run the special CGGTTS post-fit over the
    /// [PvtSolutions] and attach these solutions to the report being synthesized.
    /// This is dedicated to the local clock state,
    /// then resolved with higher precision, and typically used in remote
    pub fn nav_cggtts_solutions(&self) -> Self {
        let mut s = self.clone();
        s.analysis
            .push(QcAnalysis::CGGTTS(PvtSolutions::PvtSolutionsAll));
        s
    }
}
