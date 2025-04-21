use gnss_rtk::prelude::{Bias, Config as PVTConfig, Solver};

use crate::context::{
    navigation::{buffer::QcNavigationBuffer, NavTimeSolver},
    QcContext,
};

pub struct NullBias {}

impl Bias for NullBias {
    fn ionosphere_bias_m(&self, rtm: &gnss_rtk::prelude::BiasRuntime) -> f64 {
        0.0
    }

    fn troposphere_bias_m(&self, rtm: &gnss_rtk::prelude::BiasRuntime) -> f64 {
        0.0
    }
}

/// [NavPvtSolver] is used to resolve PVT solutions from a [QcContext].
pub struct NavPvtSolver<'a> {
    solver: Solver<QcNavigationBuffer<'a>, NullBias, NavTimeSolver>,
}

impl QcContext {
    /// Obtain [NavPvtSolver] from this [QcContext], ready to solve PVT solutions.
    /// Current [QcContext] needs to be navigation compatible.
    /// ```
    /// use gnss_qc::prelude::QcContext;
    ///
    /// // Load some data
    /// ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
    ///     .unwrap();
    ///
    /// // Navigation compatible contexts greatly enhance the reporting capability.
    /// // We can report
    /// // - the type of navigation process the data set would allow.
    /// ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
    ///     .unwrap();
    ///
    /// let mut nav_pvt = ctx.nav_pvt_solver()
    ///     .expect("This context is navigation compatible!");
    ///
    /// ```
    pub fn nav_pvt_solver<'a>(&'a self, cfg: PVTConfig) -> Option<NavPvtSolver<'a>> {
        let nav_buffer = self.navigation_buffer()?;
        let nav_time = self.nav_time_solver()?;

        let null_bias = NullBias {};

        let solver = Solver::new_almanac_frame(
            cfg,
            self.almanac.clone(),
            self.earth_cef,
            nav_buffer,
            nav_time,
            null_bias,
            None,
        );

        Some(NavPvtSolver { solver })
    }
}
