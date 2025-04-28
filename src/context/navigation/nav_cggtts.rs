use crate::{context::navigation::NavPPPSolver, prelude::QcContext};

use gnss_rtk::prelude::{Config as PPPConfig, Error as SolverError, SPEED_OF_LIGHT_M_S};

use cggtts::prelude::{
    CommonViewCalendar as CvCalendar, Observation as FitObservation, SkyTracker,
    Track as CggttsTrack,
};

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
    /// Obtain a [NavCggttsSolver] from any navigation compatible [QcContext], that
    /// you can then iterate through [CggttsTrack]s (solutions).
    pub fn nav_cggtts_solver<'a>(&'a self, cfg: PPPConfig) -> Option<NavCggttsSolver<'a>> {
        let nav_ppp = self.nav_ppp_solver(cfg)?;

        let cv_calendar = CvCalendar::bipm();

        Some(NavCggttsSolver {
            nav_ppp,
            cv_calendar,
            tracker: SkyTracker::new(),
        })
    }
}

impl<'a> Iterator for NavCggttsSolver<'a> {
    type Item = Result<CggttsTrack, SolverError>;

    /// Iterate [NavPPPSolver] and try to obtain a new [PVTSolution].
    fn next(&mut self) -> Option<Self::Item> {
        let pvt = self.nav_ppp.next()?;

        match pvt {
            Ok(pvt) => {
                for sv in pvt.sv.iter() {
                    let refsys = pvt.clock_offset_s;
                    let refsv = refsys + sv.clock_correction.unwrap_or_default().to_seconds();
                    let mdtr = sv.tropo_bias.unwrap_or_default() / SPEED_OF_LIGHT_M_S;

                    // tracking
                    let track = FitObservation {
                        epoch: pvt.epoch,
                        refsv,
                        refsys,
                        mdtr,
                        mdio: None,
                        msio: None,
                        elevation: sv.elevation,
                        azimuth: sv.azimuth,
                    };
                }
            }
            Err(e) => {
                error!("pvt solver error: {}", e);
                return Some(Err(e));
            }
        }
    }
}
