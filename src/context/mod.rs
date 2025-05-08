//! GNSS processing context definition.
use itertools::Itertools;
use std::collections::HashMap;

use crate::prelude::QcConfig;

mod rinex;

#[cfg(feature = "flate2")]
#[cfg_attr(docsrs, doc(cfg(feature = "flate2")))]
mod flate2;

#[cfg(feature = "sp3")]
#[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
mod sp3;

pub(crate) mod data;

#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
pub(crate) mod navigation;

// #[cfg(feature = "navigation")]
// #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
// pub mod time;

#[cfg(feature = "navigation")]
use crate::prelude::{Almanac, Frame};

#[cfg(doc)]
use crate::prelude::{QcPreferedIndexing, QcProductType};

// local exports
pub(crate) use data::QcDataWrapper;

// pub export
pub use data::{QcIndexing, QcProductType, QcSourceDescriptor};

/// [QcContext] is a general structure capable to store most common GNSS data.   
/// It is dedicated to post processing workflows, precise timing or atmosphere analysis.
///
/// One typical application is the synthesis of a complete analysis report.  
/// For the reason GNSS data covers a large spectrum and also, because precise applications
/// usually requires confidance on the input data quality.   
///
/// To answer this need, you can synthesize a report from [QcContext] at any point.  
/// The reported content and complexity of the task depends on:
///
/// - the available data. That is, the data you just loaded.
/// - the [QcConfig] preset
///
/// Basic example:
/// ```
/// use gnss_qc::prelude::QcContext;
///
/// let mut ctx = QcContext::new();
///
/// // The most basic would be to load some signals and verify them
/// ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
///     .unwrap();
///
/// // Navigation compatible contexts greatly enhance the reporting capability.
/// // We can report
/// // - the type of navigation process the data set would allow.
/// ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
///     .unwrap();
///
/// let report = ctx.report();
///
/// // format your report using one of the proposed methods.
/// ```
///
/// When built with SP3 option, the library allows to consider precise orbital products.
/// Reported information is naturally "enhanced":
/// ```
/// use gnss_qc::prelude::{QcContext, QcOrbitPreference};
///
/// let mut ctx = QcContext::new();
///
/// ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
///     .unwrap();
///
/// ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
///     .unwrap();
///
/// ctx.load_gzip_sp3_file("data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz")
///     .unwrap();
///
/// // When both RINEX and SP3 are present and you are interested in navigation,
/// // the Orbit source preference becomes vital and we only allow selection of either one of them.
/// ctx.configuration.set_orbit_preference(QcOrbitPreference::PreciseProducts);
///
/// // Reporting over the entire PPP compatible setup
/// let report = ctx.report();
///
/// // format your report using one of the proposed methods.
/// ```
///
/// This library allows post processed navigation as long as the "navigation" feature
/// is enabled. We integrate a NAV PVT solver that will enable solving PVT solutions
/// from the provided setup, that needs to be Navigation compatible (use the summary report to
/// verify capabilities):
///
/// ```
/// use gnss_qc::prelude::{QcContext, QcOrbitPreference};
///
/// // When built with "navigation" + "embed_ephem",
/// // It is possible to perform high precision navigation without any internet access.
/// let mut ctx = QcContext::new();
///
/// // For people that can access the internet and target ultra high precision,
/// // we recommend adding the JPL BPC database, and keep it up to date.
/// // Uncomment this line to do so.
/// // ctx.update_jpl_bpc();
///
/// // Load basic setup
///
/// // Obtain the NAV PVT solver
/// ```
///
/// PPP navigation is then achieved by running the previous example, on a PPP compatible setup:
/// ```
/// use gnss_qc::prelude::{QcContext, QcOrbitPreference};
///
/// let mut ctx = QcContext::new();
///
/// // Load PPP setup
/// // Obtain the NAV PVT solver
/// ```
#[derive(Clone)]
pub struct QcContext {
    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    /// Current [Almanac] definition.
    pub almanac: Almanac,

    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    /// ECEF [Frame] used in possible navigation processes.
    pub earth_cef: Frame,

    /// [QcConfig] preset.
    pub configuration: QcConfig,

    /// [QcDataEntry] storage
    pub(crate) data: HashMap<QcSourceDescriptor, QcDataWrapper>,
}

impl QcContext {
    /// Creates a new [QcContext] for GNSS post processing with default configuration.
    /// ```
    /// use gnss_qc::prelude::{QcContext, TimeScale};
    ///
    /// // create a new (empty) context
    /// let mut context = QcContext::new();
    ///
    /// // load some data
    /// context.load_rinex_file("data/OBS/V2/AJAC3550.21O")
    ///     .unwrap();
    ///
    /// // do something
    /// assert_eq!(context.timescale(), Some(TimeScale::GPST));
    /// ```
    pub fn new() -> Self {
        #[cfg(feature = "navigation")]
        let (almanac, earth_cef) = Self::default_almanac_frame();

        Self {
            #[cfg(feature = "navigation")]
            almanac,

            #[cfg(feature = "navigation")]
            earth_cef,

            data: Default::default(),
            configuration: QcConfig::default(),
        }
    }

    /// Obtain an [Iterator] over all [QcProductType]s present in current [QcContext].
    pub fn product_types_iter(&self) -> Box<dyn Iterator<Item = QcProductType> + '_> {
        Box::new(self.data.iter().map(|(k, _)| k.product_type).unique())
    }

    /// Build an updated [QcContext] with [QcConfig] preferences.
    /// We recommend doing this prior loading any data!
    pub fn with_configuration_preferences(&self, cfg: QcConfig) -> Self {
        let mut s = self.clone();
        s.configuration = cfg;
        s
    }
}
