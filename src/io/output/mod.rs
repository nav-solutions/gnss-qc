//! Qc pipeline output
use gnss_rs::prelude::SV;

#[cfg(feature = "nav")]
use gnss_rtk::prelude::PVTSolution;

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

    /// [SignalAnalysisResults] solved for a given signal source
    SignalAnalysis(SignalAnalysisResults),
}
