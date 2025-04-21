use crate::prelude::QcContext;
use gnss_rtk::prelude::{Epoch, Time, TimeOffset};
use qc_traits::GnssAbsoluteTime;

pub struct NavTimeSolver {
    header_served: bool,
    header_pool: Vec<TimeOffset>,
    body_iter: Box<dyn Iterator<Item = TimeOffset>>,
}

impl Time for NavTimeSolver {
    fn bdt_gpst_time_offset(&mut self, now: Epoch) -> Option<TimeOffset> {
        None
    }

    fn bdt_gst_time_offset(&mut self, now: hifitime::Epoch) -> Option<TimeOffset> {
        None
    }

    fn bdt_utc_time_offset(&mut self, now: Epoch) -> Option<TimeOffset> {
        None
    }

    fn gpst_utc_time_offset(&mut self, now: Epoch) -> Option<TimeOffset> {
        None
    }

    fn gst_gpst_time_offset(&mut self, now: Epoch) -> Option<TimeOffset> {
        None
    }

    fn gst_utc_time_offset(&mut self, now: Epoch) -> Option<TimeOffset> {
        None
    }
}

impl QcContext {
    /// Obtain [NavTimeSolver]
    pub fn nav_time_solver<'a>(&'a self) -> Option<NavTimeSolver> {
        let brdc = self.brdc_navigation()?;

        Some(NavTimeSolver {
            header_served: false,
            header_pool: Vec::new(),
            body_iter: Box::new([].into_iter()),
        })
    }
}
