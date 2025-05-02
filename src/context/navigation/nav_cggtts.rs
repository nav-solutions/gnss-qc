use thiserror::Error;

use crate::{
    context::navigation::{NavPPPSolver, SolutionsIter},
    prelude::QcContext,
};

use gnss_rtk::prelude::{
    Config as PPPConfig, Error as SolverError, User as UserProfile, SPEED_OF_LIGHT_M_S,
};

use cggtts::prelude::{
    CommonViewCalendar as CvCalendar, Observation as FitObservation, SkyTracker,
    Track as CggttsTrack,
};

/// [NavCggttsError] is returned when the CGGTTS solution solving process goes wrong.
/// It's either a GNSS-RTK internal failure, or a failure during the CGGTTS post-fit of the solution
#[derive(Error, Debug)]
pub enum NavCggttsError {
    #[error("solver error: {0}")]
    Solver(SolverError),
    #[error("Work In Progress")]
    WorkInProgress,
}

/// [NavCggttsSolver] is very similar to [NavPPPSolver] and operates identically.
///
/// The key differences are:
/// - each solution is reworked as [CGGTTS] solution (also referred to as "Tracks").   
/// This is done by apply the special [CGGTTS] tracking and fitting techniqued, specified
/// by the BIPM.
///
/// - the rover is intended to be a static target. This is a laboratory application
/// only, targgeting precise solutions.    
/// Although, we do not have guarding logic,
/// and this framework allows solving CGGTTS solutions from roaming rovers.   
/// You are responsible of correct deployment of the [NavCggttsSolver].
#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
pub struct NavCggttsSolver<'a> {
    tracker: SkyTracker,
    cv_calendar: CvCalendar,
    nav_ppp: NavPPPSolver<'a>,
}

#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
impl QcContext {
    /// Obtain a [NavCggttsSolver] from any navigation compatible [QcContext] using absolute/direct navigation
    /// (no external reference).  
    /// You can then grab the solutions with [SolutionsIter::next].
    /// ## Input
    /// - cfg: [PPPConfig] preset
    /// ## Output
    /// - solver: [NavCggttsSolver]
    pub fn nav_cggtts_ppp_solver<'a>(&'a self, cfg: PPPConfig) -> Option<NavCggttsSolver<'a>> {
        let nav_ppp = self.nav_ppp_solver(cfg)?;

        let cv_calendar = CvCalendar::bipm();

        Some(NavCggttsSolver {
            nav_ppp,
            cv_calendar,
            tracker: SkyTracker::new(),
        })
    }
}

impl<'a> SolutionsIter for NavCggttsSolver<'a> {
    type Error = NavCggttsError;
    type Solution = CggttsTrack;

    /// Iterate [NavPPPSolver] and try to obtain a new [PVTSolution].
    fn next(&mut self, user_profile: UserProfile) -> Option<Result<Self::Solution, Self::Error>> {
        // grab next PVT
        let pvt = self.nav_ppp.next(user_profile)?;

        match pvt {
            Ok(pvt) => {
                let epoch = pvt.epoch;

                for sv in pvt.sv.iter() {
                    let refsys = pvt.clock_offset_s;
                    let refsv = refsys + sv.clock_correction.unwrap_or_default().to_seconds();
                    let mdtr = sv.tropo_bias.unwrap_or_default() / SPEED_OF_LIGHT_M_S;

                    let (azimuth_deg, elevation_deg) = (sv.azimuth_deg, sv.elevation_deg);

                    // tracking
                    let track = FitObservation {
                        epoch,
                        refsv,
                        refsys,
                        mdtr,
                        mdio: 0.0,  // TODO
                        msio: None, // TODO
                        elevation: elevation_deg,
                        azimuth: azimuth_deg,
                    };
                }

                return Some(Err(NavCggttsError::WorkInProgress));
            }
            Err(e) => {
                error!("pvt solver error: {}", e);
                return Some(Err(NavCggttsError::Solver(e)));
            }
        }
    }
}
