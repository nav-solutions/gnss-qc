use crate::prelude::QcContext;
use qc_traits::GnssAbsoluteTime;

use gnss_rtk::prelude::{AbsoluteTime, Epoch, TimeScale};

pub struct NavTimeSolver {
    solver: GnssAbsoluteTime,
}

impl AbsoluteTime for NavTimeSolver {
    fn new_epoch(&mut self, now: Epoch) {
        self.solver.outdate_weekly(now);
    }

    fn epoch_correction(&self, t: Epoch, target: TimeScale) -> Epoch {
        if let Some(corrected) = self.solver.precise_epoch_correction(t, target) {
            corrected
        } else {
            t.to_time_scale(target)
        }
    }
}

impl QcContext {
    /// Obtain [NavTimeSolver]
    pub fn nav_time_solver<'a>(&'a self) -> NavTimeSolver {
        let solver = self.gnss_absolute_time_solver();
        NavTimeSolver { solver }
    }
}
