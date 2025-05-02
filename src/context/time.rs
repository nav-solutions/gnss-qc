use crate::prelude::{QcContext, TimeScale};

use qc_traits::{GnssAbsoluteTime, Merge, Timeshift};

impl QcContext {
    /// Form a [GnssAbsoluteTime] solver from this [QcContext],
    /// used to allow transposition into other [TimeScale]s.   
    /// This requires navigation feature  to be enabled and compliance to be effective.
    pub fn gnss_absolute_time_solver(&self) -> GnssAbsoluteTime {
        let mut solver = GnssAbsoluteTime::new(&[]);

        for (_, rinex) in self.brdc_navigations_rinex_iter() {
            let brdc = rinex.gnss_absolute_time_solver().unwrap(); // infaillible
            solver.merge_mut(&brdc).unwrap(); // infaillble
        }

        solver
    }

    /// Precise temporal transposition of each individual products contained in current [QcContext].
    /// NB: transposition might not be feasible for some components, therefore
    /// you should double check the newly obtained [QcContext].
    ///
    /// This may apply to your [SP3] products, if feature is activated.
    ///
    /// Example (1): precise RINEX transpositions
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
    ///     assert_eq!(t.time_scale, TimeScale::GPST);
    /// }
    ///
    /// // You need to stack NAV RINEX for that day as well
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
    /// Example (2): SP3 transposition.
    /// SP3 are totally valid in any GNSS timescale, you can use this framework
    /// to reformat as desired !
    ///
    pub fn timescale_transposition(&self, target: TimeScale) -> Self {
        let mut s = self.clone();
        s.timescale_transposition_mut(target);
        s
    }

    pub fn timescale_transposition_mut(&mut self, target: TimeScale) {
        let solver = self.gnss_absolute_time_solver();

        for (_, rinex) in self.observations_rinex_iter_mut() {
            rinex.timeshift_mut(&solver, target);
        }

        #[cfg(feature = "sp3")]
        for (_, sp3) in self.sp3_products_iter_mut() {
            sp3.timeshift_mut(&solver, target);
        }
    }
}
