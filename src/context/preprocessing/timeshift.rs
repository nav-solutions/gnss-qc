use crate::prelude::{QcContext, TimeScale, Timeshift};

impl QcContext {
    /// Temporal transposition into desired [TimeScale].
    /// The difference between this method and [Self::precise_timeshift_mut]
    /// is that we have no means to reflect the _actual_ state of each [TimeScale],
    /// while the latter can take it into account.
    /// ```
    /// use gnss_qc::prelude::*;
    ///
    /// let mut context = QcContext::new();
    ///
    /// // GPST data
    /// context.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
    ///     .unwrap();
    ///
    /// // GPST data
    /// context.load_gzip_rinex_file("data/CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz")
    ///     .unwrap();
    ///
    /// // GPST data: this conveniently applies to any supported (temporal) products
    /// context.load_gzip_sp3_file("data/SP3/C/")
    ///     .unwrap();
    ///
    /// // transpose to GST
    /// context.timeshift_mut(TimeScale::GST);
    /// ```
    pub fn timeshift_mut(&mut self, target: TimeScale) {
        for (_, data) in self.data.iter_mut() {
            if let Some(rinex) = data.as_mut_rinex() {
                rinex.timeshift_mut(solver, target);
            } else if let Some(sp3) = data.as_mut_sp3() {
                sp3.timeshift_mut(solver, target);
            }
        }
    }
    
    /// Precise temporal transposition using provided [GnssAbsoluteTime] into
    /// desired [TimeScale]. The difference between this method and [Self::timeshift_mut]
    /// is that the [GnssAbsoluteTime] solver allows to take into account a database
    /// of [TimeScale] corrections and take into account the _actual_ state of [TimeScale]s.
    ///
    /// ```
    /// use gnss_qc::prelude::*;
    ///
    /// // build a dataset
    /// let mut context = QcContext::new();
    ///
    /// // GPST data
    /// context.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
    ///     .unwrap();
    ///
    /// // GPST data
    /// context.load_gzip_rinex_file("data/CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz")
    ///     .unwrap();
    ///
    ///
    /// // transpose to GST
    /// context.timeshift_mut(TimeScale::GST);
    /// ```
    pub fn precise_timeshift_mut(&mut self, solver: &TimeCorrectionDatabase, target: TimeScale) {
        for (_, data) in self.data.iter_mut() {
            if let Some(rinex) = data.as_mut_rinex() {
                rinex.precise_timeshift_mut(solver, target);
            } else if let Some(sp3) = data.as_mut_sp3() {
                sp3.precise_timeshift_mut(solver, target);
            }
        }
    }
}
