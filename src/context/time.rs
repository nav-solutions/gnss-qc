use crate::prelude::{QcContext, TimeScale, QcIndexing};

use qc_traits::{GnssTimeCorrectionsDatabase, Merge, Timeshift};

impl QcContext {
    /// Form a [GnssAbsoluteTime] solver from this [QcContext],
    /// used in precise [TimeScale] transposition.
    /// This requires at least one BRDC RINEX file.
    /// ```
    /// use gnss_qc::prelude::*;
    /// 
    /// let mut context = QcContext::new();
    ///
    /// // GPST data
    /// context.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
    ///     .unwrap();
    ///
    /// // context is not compatible at this point
    /// assert!(context.gnss_absolute_time_solver().is_none());
    ///
    /// // Load NAV RINEX
    /// context.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
    ///     .unwrap();
    ///
    /// let solver = context.gnss_absolute_time_solver().unwrap();
    ///
    /// // you can then use this information for precise temporal transposition
    /// context.precise_timeshift_mut(&solver, TimeScale::GST);
    /// ```
    pub fn gnss_time_corrections_database(&self) -> Option<GnssTimeCorrectionsDatabase> {
        let mut ret = Option::<GnssAbsoluteTime>::None;

        for (desc, data) in self.data.iter() {
            if desc.product_type == QcProductType::BroadcastNavigation {
                let rinex = data.as_rinex()
                    .expect("internal error: RINEX data access");

                let new = rinex.gnss_time_corrections_database()
                    .expect("gnss_absolute_time should exist");

                if let Some(solver) = &mut ret {
                    solver.merge_mut(new);
                } else {
                    solver = Some(new);
                }
            }
        }

        ret
    }
    
    /// Form a [GnssTimeCorrectionsDatabase] for one agency or publisher in particular.
    /// This database can then be used in precise time correction and transposition.
    /// ```
    /// use gnss_qc::prelude::*;
    /// 
    /// let mut context = QcContext::new();
    ///
    /// // GPST data
    /// context.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
    ///     .unwrap();
    ///
    /// // context is not compatible at this point
    /// assert!(context.gnss_time_corrections_database().is_none());
    ///
    /// // Load NAV RINEX (x1)
    /// context.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
    ///     .unwrap();
    ///
    /// // Load NAV RINEX (x2)
    /// context.load_gzip_rinex_file("data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz")
    ///     .unwrap();
    ///
    /// // when several publishers exist, you can use this method
    /// // to select one in particular
    /// let solver = context.gnss_time_corrections_agency_database("MOJN").unwrap();
    ///
    /// // you can then use this information for precise temporal transposition
    /// context.precise_timeshift_mut(&solver, TimeScale::GST);
    /// ```
    pub fn gnss_time_corrections_agency_database(&self, agency: &str) -> Option<GnssTimeCorrectionsDatabase> {
        let filter = QcIndexing::from_agency(agency);

        for (desc, data) in self.data.iter() {
            if desc.product_type == QcProductType::BroadcastNavigation && desc.indexing == filter {
                let rinex = data.as_rinex()
                    .expect("internal error: RINEX data access");

                let solver = rinex.gnss_absolute_time_solver()?;
                return Some(solver);
            }
        }
        None
    }
}
