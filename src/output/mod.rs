#[cfg(feature = "nav")]
use gnss_rtk::prelude::PVTSolution;

#[cfg(feature = "cggtts")]
use cggtts::prelude::Track;

#[cfg(feature = "nav")]
use anise::prelude::Orbit;

#[cfg(feature = "nav")]
pub struct RoverPVTSolution {
    /// Name of selected rover. This may apply to either static or roaming rover,
    /// depending on the user application.
    pub rover: String,

    /// Resolved [PVTSolution]
    pub pvt_solution: PVTSolution,
}

#[cfg(feature = "cggtts")]
pub struct CggttsTrack {
    /// Name of selected receiver (or "rover"). This usually applies to static applications.
    pub name: String,

    /// CGGTTS solution formed as [CggttsTrack]
    pub cggtts_solution: Track,
}

#[cfg(feature = "nav")]
pub struct KeplerSolverResults {
    /// Name of given satellite
    pub satellite: String,

    /// Resolved [Orbit]al state
    pub state: Orbit,
}

pub struct SignalAnalysisResults {
    /// Signal name
    pub signal: String,
    
    /// Signal source
    pub source: SV,

    /// S/NR in dB
    pub snr_db: Option<f32>,
}

pub enum OutputItem {
    /// [PVTSolution] solved for pre selected rover
    #[cfg(feature = "nav")]
    PVTSolution(RoverPVTSolution),
    
    /// [KeplerSolverResults] solved for given ephemeris source
    #[cfg(feature = "nav")]
    KeplerSolver(KeplerSolverResults),

    /// [CggttsTrack] solved for pre selected receiver (static rover)
    #[cfg(all(feature = "cggtts", feature = "nav")]
    CggttsTrack(CggttsTrack),
    
    /// [SignalAnalysisResults] solved for a given signal source
    SignalAnalysis(SignalAnalysisResults),
}
