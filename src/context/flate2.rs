use rinex::Rinex;

use crate::{error::QcError, prelude::QcContext};

use std::path::Path;

#[cfg(feature = "sp3")]
use crate::prelude::SP3;

impl QcContext {
    /// Load a Gzip compressed RINEX file from readable [Path].
    /// ```
    /// use gnss_qc::prelude::QcContext;
    ///
    /// let mut ctx = QcContext::default();
    ///
    /// // load compressed RINEX
    /// ctx.load_gzip_rinex_file("data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz")
    ///     .unwrap();
    /// ```
    ///
    /// CRINEX (compressed RINEX) support is built-in:
    /// ```
    /// use gnss_qc::prelude::QcContext;
    ///
    /// let mut ctx = QcContext::default();
    ///
    /// // load compressed CRINEX
    /// ctx.load_gzip_rinex_file("data/CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz")
    ///     .unwrap();
    /// ```
    pub fn load_gzip_rinex_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), QcError> {
        let rinex = Rinex::from_gzip_file(&path)?;
        self.load_rinex(path, rinex)
    }

    #[cfg(feature = "sp3")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
    /// Load a Gzip compressed [SP3] file from readable [Path].
    /// ```
    /// use gnss_qc::prelude::QcContext;
    ///
    /// let mut ctx = QcContext::default();
    ///
    /// // load compressed SP3
    /// ctx.load_gzip_sp3_file("data/SP3/D/COD0MGXFIN_20230500000_01D_05M_ORB.SP3.gz")
    ///     .unwrap();
    ///
    /// assert!(ctx.has_sp3_data());
    /// ```
    pub fn load_gzip_sp3_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), QcError> {
        let sp3 = SP3::from_gzip_file(&path)?;
        self.load_sp3(path, sp3)
    }
}
