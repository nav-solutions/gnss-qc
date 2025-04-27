use crate::prelude::{QcContext, TimeScale};
use hifitime::{Duration, Polynomial};
use qc_traits::{GnssAbsoluteTime, TimePolynomial, Timeshift};

impl QcContext {
    /// Form a [GnssAbsoluteTime] solver from this [QcContext],
    /// used to allow transposition into other [TimeScale]s.
    /// This requires navigation both feature and compatibility to truly be effective.
    pub fn gnss_absolute_time_solver(&self) -> GnssAbsoluteTime {
        let mut polynomials = Vec::<TimePolynomial>::new();

        if let Some(brdc) = self.brdc_navigation() {
            if let Some(brdc) = &brdc.header.nav {
                for time_offset in brdc.time_offsets.iter() {
                    polynomials.push(TimePolynomial::from_reference_time_of_week_nanos(
                        time_offset.t_ref.0,
                        time_offset.t_ref.1,
                        time_offset.lhs,
                        time_offset.rhs,
                        Polynomial {
                            constant: Duration::from_seconds(time_offset.polynomial.0),
                            rate: Duration::from_seconds(time_offset.polynomial.1),
                            accel: Duration::from_seconds(time_offset.polynomial.2),
                        },
                    ));
                }
            }

            for (_, time_offset) in brdc.nav_system_time_frames_iter() {
                polynomials.push(TimePolynomial::from_reference_time_of_week_nanos(
                    time_offset.t_ref.0,
                    time_offset.t_ref.1,
                    time_offset.lhs,
                    time_offset.rhs,
                    Polynomial {
                        constant: Duration::from_seconds(time_offset.polynomial.0),
                        rate: Duration::from_seconds(time_offset.polynomial.1),
                        accel: Duration::from_seconds(time_offset.polynomial.2),
                    },
                ));
            }
        }

        GnssAbsoluteTime::new(&polynomials)
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
    /// // For this to work, Observations are not enough.
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

        if let Some(observations) = self.observation_mut() {
            observations.timeshift_mut(&solver, target);
        }

        #[cfg(feature = "sp3")]
        if let Some(sp3) = self.sp3_mut() {
            sp3.timeshift_mut(&solver, target);
        }
    }
}
