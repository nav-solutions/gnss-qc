/// [QcAnalysisBuilder] is used to describe and perform
/// several analysis at once.
#[derive(Default, Debug, Copy, Clone)]
pub struct QcAnalysisBuilder {
    /// True if RTK summary is to be gathered and formed.
    rtk_summary: bool,
    
    /// True if RINEX summary is to be gathered and formed.
    rinex_summary: bool,
    
    /// True if SP3 summary is to be gathered and formed.
    sp3_summary: bool,
    
    /// True if carrier phase observations view is requested.
    phase_observations: bool,

    /// True if pseudo range  observations view is requested.
    pseudo_range_observations: bool,

    /// True if doppler observations view is requested.
    doppler_observations: bool,
    
    /// True if signal power observations view is requested.
    signal_power_observations: bool,
    
    /// True when running the sampling gap histogram
    /// over Pseudo range observations
    pseudo_range_sampling_gap_histogram: bool,

    /// True when running the sampling gap histogram
    /// over Carrier phase measurements
    carrier_phase_sampling_gap_histogram: bool,
    
    /// True when running the sampling gap histogram
    /// over Doppler measurements
    doppler_sampling_gap_histogram: bool,
    
    /// True when IF combination should be formed
    pseudo_range_if_combination: bool,
    
    /// True when GF combination should be formed.
    pseudo_range_gf_combination: bool,
    
    /// True when IF combination should be formed.
    carrier_phase_if_combination: bool,
    
    /// True when GF combination should be formed.
    carrier_phase_gf_combination: bool,

    /// True when MW combination should be formed.
    melbourne_wubbena_combination: bool,

    /// True when MP analysis was requested.
    multipath: bool,

    /// True when Broadcast clock offsets should be reported
    broadcast_clock_offsets: bool,

    /// True when Broadcast clock drifts should be reported
    broadcast_clock_drifts: bool,

    /// True when Precise clock offsets should be reported
    precise_clock_offsets: bool,

    /// True when precise clock drifts should be reported
    precise_clock_drifts: bool,
    
    /// True when NAVI plot is requested
    navi_plot: bool,

    /// True when SP3 orbit projection is requested
    sp3_orbit_proj: bool,

    /// True when Orbit residuals were requested
    orbit_residuals: bool,

    /// True when SP3 temporal residuals were requested
    sp3_temporal_residuals: bool,

    /// True when NAV PVT solutions were requested
    nav_pvt_solutions: bool,

    /// True when NAV CGGTTS solutions were requested
    nav_cggtts_solutions: bool,

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
            .all_sampling_gap_histograms()
            .clock_residuals();

        #[cfg(feature = "sp3")]
        let s = s.sp3_summary()
            .orbit_residuals()
            .sp3_temporal_residuals();

        #[cfg(feature = "navigation")]
        let s = s.navi_plot()
                .nav_pvt_solutions();

        #[cfg(all(feature = "navigation", feature = "cggtts"))]
        let s = s.nav_cggtts_solutions();

        s
    }

    /// Generate a summary table
    /// (as an introduction) for all RINEX and SP3 products.
    /// This gives high level information such as timescale being used
    /// and information about the data production setup.
    pub fn all_summaries(mut self) -> Self {
        self = self.rinex_summary().rtk_summary();

        // TODO #[cfg(feature = "sp3")]
        self = self.sp3_summary();

        self
    }

    /// Generate a cartesian 2D projection
    /// of each observation per data source, per individual Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    pub fn all_observations(mut self) -> Self {
        self.phase_observations()
            .doppler_observations()
            .pseudo_range_observations()
            .signal_power_observations()
    }

    pub fn all_combinations(mut self) -> Self {
        self.all_carrier_phase_combinations()
            .all_pseudo_range_combinations()
            .melbourne_wubbena_combination()
    }

    pub fn all_carrier_phase_combinations(mut self) -> Self {
        self.carrier_phase_gf_combination()
            .carrier_phase_if_combination()
    }
    
    pub fn all_pseudo_range_combinations(mut self) -> Self {
        self.pseudo_range_gf_combination()
            .pseudo_range_if_combination()
    }

    pub fn melbourne_wubbena_combination(mut self) -> Self {
        self.melbourne_wubbena_combination = true;
        self
    }

    pub fn all_sampling_gap_histograms(mut self) -> Self {
        self.pseudo_range_sampling_gap_histogram()
            .carrier_phase_sampling_gap_histogram()
            .doppler_sampling_gap_histogram()
    }

    /// Run a sampling gap histogram analysis over pseudo-range observations.
    pub fn pseudo_range_sampling_gap_histogram(mut self) -> Self {
        self.pseudo_range_sampling_gap_histogram = true;
        self
    }
    
    /// Run a sampling gap histogram analysis over carrier phase observations.
    pub fn carrier_phase_sampling_gap_histogram(mut self) -> Self {
        self.carrier_phase_sampling_gap_histogram = true;
        self
    }
    
    /// Run a sampling gap histogram analysis over doppler observations.
    pub fn doppler_sampling_gap_histogram(mut self) -> Self {
        self.doppler_sampling_gap_histogram = true;
        self
    }

    /// [QcAnalysis::GeometryFreeCombination] will generate a cartesian 2D
    /// projection of the GF combination, per data source, per individual Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    pub fn pseudo_range_gf_combination(mut self) -> Self {
        self.pseudo_range_gf_combination = true;
        self
    }

    pub fn carrier_phase_gf_combination(mut self) -> Self {
        self.carrier_phase_gf_combination = true;
        self
    }

    /// [QcAnalysis::IonosphereFreeCombination] will generate a cartesian 2D
    /// projection of the IF combination, per data source, per individual Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    pub fn pseudo_range_if_combination(mut self) -> Self {
        self.pseudo_range_if_combination = true;
        self
    }
    
    pub fn carrier_phase_if_combination(mut self) -> Self {
        self.carrier_phase_if_combination = true;
        self
    }

    /// [QcAnalysis::MelbourneWubbenaCombination] will generate a cartesian 2D
    /// projection of the MW combination, per data source, per individual Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    pub fn melbourne_wubbena_combination(mut self) -> Self {
        self.melbourne_wubbena = true;
        self
    }

    /// [QcAnalysis::MultiPath] will generate a cartesian 2D
    /// projection of the multipath bias, for each SV and per Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    pub fn multipath(mut self) -> Self {
        self.multipath = true;
        self
    }

    /// [QcAnalysis::PhaseObservations] will generate a cartesian 2D
    /// projection of each carrier phase observation per data source, per individual Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    pub fn phase_observations(mut self) -> Self {
        self.phase_observations = true;
        self
    }

    /// [QcAnalysis::PseudoRangeObservations] will generate a cartesian 2D
    /// projection of each pseudo-range observation per data source, per individual Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    pub fn pseudo_range_observations(mut self) -> Self {
        self.pseudo_range_observations = true;
        self
    }

    /// [QcAnalysis::DopplerObservations] will generate a cartesian 2D
    /// projection of each doppler observation per data source, per individual Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    pub fn doppler_observations(mut self) -> Self {
        self.doppler_observations = true;
        self
    }

    /// [QcAnalysis::SignalPowerObservations] will generate a cartesian 2D
    /// projection of each signal reception power, per SV and Constellation.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    pub fn signal_power_observations(mut self) -> Self {
        self.signal_power_observations = true;
        self
    }

    /// Generate a cartesian 2D
    /// projection of the residual C(a) - C(b) per signal, per constellation
    /// and between each (a)-(b) data source combination.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    ///
    /// [QcAnalysis::PhaseResiduals] will generate a cartesian 2D
    /// projection of the L(a) - L(b) residual per signal, per constellation
    /// and between each (a)-(b) data source combination.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    pub fn pseudo_range_residuals(mut self) -> Self {
        self.pseudo_range_residuals = true;
        self
    }

    /// Generate a cartesian 2D
    /// projection of the residual L(a) - L(b) per signal, per constellation
    /// and between each (a)-(b) data source combination.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    /// when focusing a specific SV or Constellations.
    ///
    /// [QcAnalysis::PseudoRangeResiduals] will generate a cartesian 2D
    /// projection of the C(a) - C(b) residual per signal, per constellation
    /// and between each (a)-(b) data source combination.
    /// Because this can be quite lengthy on multi-GNSS, we recommend using this
    pub fn phase_residuals(mut self) -> Self {
        self.carrier_phase_residuals = true;
        self
    }

    /// Generate a cartesian 2D projection
    /// of meteo sensor measurements, in case such data was provided.
    /// [QcAnalysis::MeteoObservations] will generate a cartesian 2D projection
    /// of meteo sensors, in case such data was provided.
    pub fn meteo_observations(mut self) -> Self {
        self.meteo_observations = true;
        self
    }

    /// Evaluate and render (as cartesian
    /// 2D projections) the residual error between the clock states resolved
    /// from Broadcast radio message, and post-processed products (in case
    /// such have been loaded).
    ///
    /// [QcAnalysis::ClockResiduals] will evaluate and render (as cartesian
    /// 2D projections) the residual error between the clock states resolved
    /// from Broadcast radio message, and post-processed products (in case
    /// such have been loaded).
    pub fn clock_residuals(mut self) -> Self {
        self.clock_residuals = true;
        self
    }

    #[cfg(feature = "navigation")]
    /// Regroups projections and visualizations
    /// dedicated to in-depth analysis of the navigation conditions.
    /// This is the combination of the signal sampling conditions,
    /// skyview, navigation message, possible precise products
    /// and correction messages.
    ///
    /// [QcAnalysis::NaviPlot] regroups projections and visualizations
    /// dedicated to fine analysis of the navigation conditions.
    /// This is the combination of the signal sampling conditions,
    /// skyview, navigation message, possible precise products
    /// and correction messages.
    pub fn navi_plot(mut self) -> Self {
        self.navi_plot = true;
        self
    }

    #[cfg(feature = "sp3")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
    /// Generate a summary table
    /// for each SP3 product, giving high level information like
    /// data producer, reference frame and coordinates system, or timescale
    /// being used.
    /// 
    /// [QcAnalysis::SP3Summary] will generate a summary table
    /// for each SP3 product, giving high level information like
    /// data producer, reference frame and coordinates system, or timescale
    /// being used.
    pub fn sp3_summary(mut self) -> Self {
        self.sp3_summary = true;
        self
    }

    #[cfg(feature = "sp3")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
    /// Evaluate and render (as cartesian
    /// 2D/3D projections) the residual error between the orbital states resolved
    /// from Broadcast radio message, and post-processed products (in case
    /// such have been loaded).
    ///
    /// [QcAnalysis::OrbitResiduals] will evaluate and render (as cartesian
    /// 2D/3D projections) the residual error between the orbital states resolved
    /// from Broadcast radio message, and post-processed products (in case
    /// such have been loaded).
    pub fn orbit_residuals(mut self) -> Self {
        self.orbit_residuals = true;
        self
    }

    #[cfg(feature = "sp3")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
    /// Evaluate and render (as cartesian
    /// 2D projections) the residual error between the clock states resolved
    /// from Broadcast radio message, and the post-processed clock state obtained
    /// from SP3 (in case such have been loaded).
    ///
    /// [QcAnalysis::SP3TemporalResiduals] will evaluate and render (as cartesian
    /// 2D projections) the residual error between the clock states resolved
    /// from Broadcast radio message, and the post-processed clock state obtained
    /// from SP3 (in case such have been loaded).
    pub fn sp3_temporal_residuals(mut self) -> Self {
        self.sp3_temporal_residuals = true;
        self
    }

    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    /// Attach [PvtSolutions] to this report being synthesized.
    /// [PvtSolutions] regroups several projections and a lot of information,
    /// about the receiver, receiving conditions and local environment.
    ///
    /// [QcAnalysis::PVT] will attach [PvtSolutions] to this report being synthesized.
    /// [PvtSolutions] regroups several projections and a lot of information,
    /// about the receiver, receiving conditions and local environment.
    pub fn nav_pvt_solutions(mut self) -> Self {
        self.nav_pvt_solutions = true;
        self
    }

    #[cfg(all(feature = "navigation", feature = "cggtts"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "navigation", feature = "cggtts"))))]
    /// Run the special CGGTTS post-fit over the
    /// [PvtSolutions] and attach these solutions to the report being synthesized.
    /// This is dedicated to the local clock state,
    /// then resolved with higher precision, and typically used in remote
    ///
    /// [QcAnalysis::CGGTTS] will run the special CGGTTS post-fit over the
    /// [PvtSolutions]. This is dedicated to the local clock state,
    /// then resolved with higher precision, and typically used in remote
    /// (post-processed) clock synchronization.
    pub fn nav_cggtts_solutions(mut self) -> Self {
        self.nav_cggtts_solutions = true;
        self
    }

    pub fn sp3_orbit_proj(mut self) -> Self {
        self.sp3_orbit_proj = true;
        self
    }

    /// Generate a summary table
    /// (as an introduction) for all RINEX products.
    /// This gives high level information such as timescale being used
    /// and information about the data production setup.
    ///
    /// [QcAnalysis::RINEXSummary] will generate a summary table
    /// (as an introduction) for all RINEX products.
    /// This gives high level information such as timescale being used
    /// and information about the data production setup.
    pub fn rinex_summary(mut self) -> Self {
        self.rinex_summary = true;
        self
    }

    /// [QcAnalaysis::RTKSummary] is meaningful as long as you
    /// loaded two different RINEX sources. The ROVER/Base definitions
    /// do not have to be complete, because this library considers
    /// data-sources as Base by default. [QcAnalysis::RTKSummary]
    /// gives you a better understanding and projection of the RTK regional area.
    pub fn rtk_summary(mut self) -> Self {
        self.rtk_summary = true;
        self
    }

    pub(crate) fn needs_signals_buffering(&self) -> bool {
        self.nav_pvt_solutions
            | self.nav_cggtts_solutions
            | self.ionosphere_free_combination
            | self.geometry_free_combination
            | self.multipath
            | self.pseudo_range_residuals
            | self.carrier_phase_residuals
            | self.pseudo_range_sampling_gap_histogram
            | self.carrier_phase_sampling_gap_histogram
            | self.doppler_sampling_gap_histogram
    }

    pub(crate) fn needs_ephemeris_buffering(&self) -> bool {
        self.nav_pvt_solutions
            | self.nav_cggtts_solutions
            | self.orbit_residuals
            | self.clock_residuals
            | self.sp3_temporal_residuals
    }
}
