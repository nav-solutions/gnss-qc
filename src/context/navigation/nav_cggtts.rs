use crate::{context::navigation::NavPvtSolver, prelude::QcContext};

use gnss_rtk::prelude::Config as PVTConfig;

use cggtts::prelude::SkyTracker;

/// [NavCggttsSolver] is very similar to [NavPvtSolver] and operates identically.
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
    nav_pvt: NavPvtSolver<'a>,
    tracker: SkyTracker,
}

#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
impl QcContext {
    /// Obtain a [NavCggttsSolver] from any navigation compatible [QcContext], ready to
    /// solve [CGGTTS] solutions.
    pub fn nav_cggtts_solver<'a>(&'a self, cfg: PVTConfig) -> Option<NavCggttsSolver<'a>> {
        let nav_pvt = self.nav_pvt_solver(cfg)?;

        Some(NavCggttsSolver {
            nav_pvt,
            tracker: SkyTracker::new(),
        })
    }
}
