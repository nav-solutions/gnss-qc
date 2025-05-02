use crate::prelude::{QcContext, TimeScale};

use qc_traits::{GnssAbsoluteTime, Merge, Timeshift};

impl QcContext {
    /// Form a [GnssAbsoluteTime] solver from this [QcContext],
    /// used to allow transposition into other [TimeScale]s.   
    /// This requires navigation feature  to be enabled and compliance to be effective.
    pub fn gnss_absolute_time_solver(&self) -> GnssAbsoluteTime {
        let mut solver = GnssAbsoluteTime::new(&[]);

        if let Some(rinex) = &self.brdc_navigation {
            let brdc = rinex.gnss_absolute_time_solver().unwrap(); // infaillible
            solver.merge_mut(&brdc).unwrap(); // infaillible
        }

        solver
    }

    /// Precise temporal transposition of each individual products contained in current [QcContext].
    ///
    /// NB: transposition might not be feasible for some components, therefore
    /// you should double check the newly obtained [QcContext].
    ///
    /// This may apply to [SP3] products, if feature is activated.
    ///
    /// Example (1): RINEX transposition
    /// ```
    /// use gnss_qc::prelude::{QcContext, TimeScale};
    ///
    /// let mut context = QcContext::new();
    ///
    /// // GPST observations
    /// context.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
    ///     .unwrap();
    ///
    /// // Transposition attempt
    /// let transposed = context.timescale_transposition(TimeScale::GST);
    /// let transposed_obs = transposed.observation().unwrap();
    ///
    /// // For this to work, Observations are not enough.
    /// for t in transposed_obs.epoch_iter() {
    ///     assert_eq!(t.time_scale, TimeScale::GST);
    /// }
    /// ```
    ///
    /// When BRDC Navigation RINEX is provided, we can take advantage of it, to apply
    /// a more precise transposition, as this type of RINEX may describe conversion methods
    /// to actual true state of specific timescales.
    ///
    /// In this example, this applies to GPST, UTC and GST. Any transposition
    /// to those timescale will be more accurate and follow the actual timescale state:
    /// ```
    /// use gnss_qc::prelude::{QcContext, TimeScale};
    ///
    /// let mut context = QcContext::new();
    ///
    /// // GPST observations
    /// context.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
    ///     .unwrap();
    ///
    /// // NAV BRDC RINEX
    /// context.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
    ///     .unwrap();
    ///
    /// let transposed = context.timescale_transposition(TimeScale::GST);
    /// let transposed_obs = transposed.observation().unwrap();
    ///
    /// // Verify transposition is now effective
    /// for t in transposed_obs.epoch_iter() {
    ///     assert_eq!(t.time_scale, TimeScale::GST);
    /// }
    /// ```
    ///
    /// Example: SP3 transposition is totally valid.
    ///
    pub fn timescale_transposition(&self, target: TimeScale) -> Self {
        let mut s = self.clone();
        s.timescale_transposition_mut(target);
        s
    }

    pub fn timescale_transposition_mut(&mut self, target: TimeScale) {
        let solver = self.gnss_absolute_time_solver();

        for (_, rinex) in self.observation_sources_iter_mut() {
            rinex.timeshift_mut(&solver, target);
        }

        #[cfg(feature = "sp3")]
        if let Some(sp3) = &mut self.sp3 {
            sp3.timeshift_mut(&solver, target);
        }
    }
}
