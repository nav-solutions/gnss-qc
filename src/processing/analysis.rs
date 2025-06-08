/// All supported [QcAnalysis]
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum QcAnalysis {
    /// [QcAnalysis::RINEXSummary] will generate a summary table
    /// (as an introduction) for all RINEX products.
    /// This gives high level information such as timescale being used
    /// and information about the data production setup.
    RINEXSummary,

    /// [QcAnalysis::SP3Summary] will generate a summary table
    /// for each SP3 product, giving high level information like
    /// data producer, reference frame and coordinates system, or timescale
    /// being used.
    #[cfg(feature = "sp3")]
    SP3Summary,

    /// [QcAnalaysis::RTKSummary] is meaningful as long as you
    /// loaded two different RINEX sources. The ROVER/Base definitions
    /// do not have to be complete, because this library considers
    /// data-sources as Base by default. [QcAnalysis::RTKSummary]
    /// gives you a better understanding and projection of the RTK regional area.
    RTKSummary,

    /// [QcAnalysis::PseudoRangeObservations] will generate a cartesian 2D
    /// projection of each pseudo-range observation per data source, per individual Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    PseudoRangeObservations,

    /// [QcAnalysis::PhaseObservations] will generate a cartesian 2D
    /// projection of each carrier phase observation per data source, per individual Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    PhaseObservations,

    /// [QcAnalysis::DopplerObservations] will generate a cartesian 2D
    /// projection of each doppler observation per data source, per individual Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    DopplerObservations,

    /// [QcAnalysis::SignalPowerObservations] will generate a cartesian 2D
    /// projection of each signal reception power, per SV and Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    SignalPowerObservations,

    /// [QcAnalysis::MeteoObservations] will generate a cartesian 2D projection
    /// of meteo sensors, in case such data was provided.
    MeteoObservations,

    /// [QcAnalysis::PseudoRangeResiduals] will generate a cartesian 2D
    /// projection of the C(a) - C(b) residual per signal, per constellation
    /// and between each (a)-(b) data source combination.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    PseudoRangeResiduals,

    /// [QcAnalysis::PhaseResiduals] will generate a cartesian 2D
    /// projection of the L(a) - L(b) residual per signal, per constellation
    /// and between each (a)-(b) data source combination.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    PhaseResiduals,

    /// [QcAnalysis::IonosphereFreeCombination] will generate a cartesian 2D
    /// projection of the IF combination, per data source, per individual Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    IonosphereFreeCombination,

    /// [QcAnalysis::GeometryFreeCombination] will generate a cartesian 2D
    /// projection of the GF combination, per data source, per individual Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    GeometryFreeCombination,

    /// [QcAnalysis::MelbourneWubbenaCombination] will generate a cartesian 2D
    /// projection of the MW combination, per data source, per individual Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    MelbourneWubbenaCombination,

    /// [QcAnalysis::MultiPath] will generate a cartesian 2D
    /// projection of the multipath bias, for each SV and per Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    MultiPath,

    /// [QcAnalysis::ClockResiduals] will evaluate and render (as cartesian
    /// 2D projections) the residual error between the clock states resolved
    /// from Broadcast radio message, and post-processed products (in case
    /// such have been loaded).
    ClockResiduals,

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
            Self::RINEXSummary => write!(f, "RINEX Summary"),

            #[cfg(feature = "sp3")]
            Self::SP3Summary => write!(f, "SP3 Summary"),

            Self::RTKSummary => write!(f, "RTK Summary"),
            Self::NaviPlot => write!(f, "NAVI Plot"),
            Self::PseudoRangeObservations => write!(f, "Pseudo Range Observations"),
            Self::PseudoRangeResiduals => write!(f, "Pseudo Range Residuals"),
            Self::PhaseObservations => write!(f, "Carrier Phase Observations"),
            Self::PhaseResiduals => write!(f, "Carrier Phase Residuals"),
            Self::DopplerObservations => write!(f, "Doppler Shift Observations"),
            Self::GeometryFreeCombination => write!(f, "Geometry Free Combination"),
            Self::IonosphereFreeCombination => write!(f, "Ionosphere Free Combination"),
            Self::SignalPowerObservations => write!(f, "Signal Power Observations"),
            Self::ClockResiduals => write!(f, "Clock Residuals"),
            Self::MeteoObservations => write!(f, "Meteo Observations"),
            Self::MultiPath => write!(f, "Multipath Analysis"),
            Self::MelbourneWubbenaCombination => write!(f, "Melbourne Wübbena Combination"),

            #[cfg(feature = "sp3")]
            Self::SP3TemporalResiduals => write!(f, "SP3 Clock Residuals"),

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
    /// We do not recommend to use this in Multi-GNSS multi-day surveying,
    /// because rendering time (especially for signals projection) is then quite long
    pub fn all() -> Self {
        let s = Self::default()
            .all_summaries()
            .all_observations()
            .all_combinations()
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
    /// (as an introduction) for all RINEX and SP3 products.
    /// This gives high level information such as timescale being used
    /// and information about the data production setup.
    pub fn all_summaries(&self) -> Self {
        let s = self.rinex_summary().rtk_summary();

        #[cfg(feature = "sp3")]
        let s = s.sp3_summary();

        s
    }

    /// Generate a cartesian 2D projection
    /// of each observation per data source, per individual Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    pub fn all_observations(&self) -> Self {
        self.phase_observations()
            .doppler_observations()
            .pseudo_range_observations()
            .signal_power_observations()
    }

    pub fn all_combinations(&self) -> Self {
        self.geometry_free_combination()
            .ionosphere_free_combination()
            .melbourne_wubbena_combination()
    }

    pub fn geometry_free_combination(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::GeometryFreeCombination);
        s
    }

    pub fn ionosphere_free_combination(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::IonosphereFreeCombination);
        s
    }

    pub fn melbourne_wubbena_combination(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::MelbourneWubbenaCombination);
        s
    }

    pub fn multipath(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::MultiPath);
        s
    }

    pub fn phase_observations(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::PhaseObservations);
        s
    }

    pub fn pseudo_range_observations(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::PseudoRangeObservations);
        s
    }

    pub fn doppler_observations(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::DopplerObservations);
        s
    }

    pub fn signal_power_observations(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::SignalPowerObservations);
        s
    }

    /// Generate a cartesian 2D
    /// projection of the residual C(a) - C(b) per signal, per constellation
    /// and between each (a)-(b) data source combination.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    pub fn pseudo_range_residuals(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::PseudoRangeResiduals);
        s
    }

    /// Generate a cartesian 2D
    /// projection of the residual L(a) - L(b) per signal, per constellation
    /// and between each (a)-(b) data source combination.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    pub fn phase_residuals(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::PhaseResiduals);
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

    /// Generate a summary table
    /// (as an introduction) for all RINEX products.
    /// This gives high level information such as timescale being used
    /// and information about the data production setup.
    pub fn rinex_summary(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::RINEXSummary);
        s
    }

    pub fn rtk_summary(&self) -> Self {
        let mut s = self.clone();
        s.analysis.push(QcAnalysis::RTKSummary);
        s
    }
}
