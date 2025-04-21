use gnss_rtk::prelude::{Bias, Config as PVTConfig, OrbitSource, Solver, Time};

use crate::context::{navigation::buffer::QcNavigationBuffer, QcContext};

pub struct NullBias {}
pub struct NullTime {}

impl Bias for NullBias {
    fn ionosphere_bias_m(&self, rtm: &gnss_rtk::prelude::BiasRuntime) -> f64 {
        0.0
    }

    fn troposphere_bias_m(&self, rtm: &gnss_rtk::prelude::BiasRuntime) -> f64 {
        0.0
    }
}

impl Time for NullTime {
    fn bdt_gpst_time_offset(
        &mut self,
        now: hifitime::Epoch,
    ) -> Option<gnss_rtk::prelude::TimeOffset> {
        None
    }

    fn bdt_gst_time_offset(
        &mut self,
        now: hifitime::Epoch,
    ) -> Option<gnss_rtk::prelude::TimeOffset> {
        None
    }

    fn bdt_utc_time_offset(
        &mut self,
        now: hifitime::Epoch,
    ) -> Option<gnss_rtk::prelude::TimeOffset> {
        None
    }

    fn gpst_utc_time_offset(
        &mut self,
        now: hifitime::Epoch,
    ) -> Option<gnss_rtk::prelude::TimeOffset> {
        None
    }

    fn gst_gpst_time_offset(
        &mut self,
        now: hifitime::Epoch,
    ) -> Option<gnss_rtk::prelude::TimeOffset> {
        None
    }

    fn gst_utc_time_offset(
        &mut self,
        now: hifitime::Epoch,
    ) -> Option<gnss_rtk::prelude::TimeOffset> {
        None
    }
}

/// [NavPvtSolver] is used to resolve PVT solutions from a [QcContext].
pub struct NavPvtSolver<'a> {
    solver: Solver<QcNavigationBuffer<'a>, NullBias, NullTime>,
}

impl QcContext {
    /// Obtain [NavPvtSolver] from this [QcContext], ready to solve PVT solutions.
    /// Current [QcContext] needs to be navigation compatible.
    pub fn nav_pvt_solver<'a>(&'a self, cfg: PVTConfig) -> Option<NavPvtSolver<'a>> {
        let nav_buffer = self.navigation_buffer()?;

        let null_time = NullTime {};
        let null_bias = NullBias {};

        let solver = Solver::new_almanac_frame(
            cfg,
            self.almanac.clone(),
            self.earth_cef,
            nav_buffer,
            null_time,
            null_bias,
            None,
        );

        Some(NavPvtSolver { solver })
    }
}
