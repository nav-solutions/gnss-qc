//! Navigation: P.V.T solutions solving, powered by GNSS-RTK.
//!
//! ```text
//!  OBS (rover) ─────┐
//!  OBS (RTK base) ──┤                  ┌─ candidates (rover / base)
//!  NAV ─────────────┼─► QcEngine ──► QcNavigationProcessor ──► PVT solutions (stream)
//!  SP3 ─────────────┘   (synchronized   └─ orbits + clocks: SP3 > NAV, per satellite
//!                        context)
//! ```
//!
//! Orbital products: [QcOrbitPreference] defines how precise (SP3) and broadcast (NAV)
//! products are combined. The default ([QcOrbitPreference::Auto]) always prefers precise
//! products, per satellite and per epoch, and falls back to broadcast ephemerides when
//! precise products do not cover the satellite (missing satellite, end of file, data gap).
mod candidates;
mod kepler;
mod processor;
mod provider;
mod rtk;

pub use candidates::{build_candidates, rtk_carrier};
pub use kepler::{relativistic_correction_s, QcKepler, QcKeplerState, SPEED_OF_LIGHT_M_S};
pub use processor::{
    QcNavigationConfig, QcNavigationError, QcNavigationProcessor, QcNavigationTechnique,
};
pub use provider::{
    QcClockProvider, QcEnvironment, QcNavigationData, QcNullEphemeris, QcOrbitCounters,
    QcOrbitKind, QcOrbitPreference, QcOrbitProvider, QcTime,
};
pub use rtk::QcRtkBase;

// re-export: solver configuration
pub use gnss_rtk::prelude::{
    Almanac, ClockProfile, Config as QcSolverConfig, Method, PVTSolution, PVTSolutionType,
    UserParameters, UserProfile,
};
