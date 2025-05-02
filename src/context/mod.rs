//! GNSS processing context definition.
use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use qc_traits::{Filter, Preprocessing, Repair, RepairTrait};

use crate::{
    error::QcError,
    prelude::{Constellation, QcConfig, QcProductType, Rinex, TimeScale},
};

mod data;
mod indexing;
mod key;
mod rinex;

use data::QcData;

pub(crate) use key::QcDataKey;

pub use indexing::QcIndexing;

#[cfg(feature = "flate2")]
#[cfg_attr(docsrs, doc(cfg(feature = "flate2")))]
mod flate2;

#[cfg(feature = "sp3")]
#[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
mod sp3;

#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
pub(crate) mod navigation;

#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
pub mod time;

#[cfg(feature = "navigation")]
use crate::prelude::{Almanac, Frame};

#[cfg(doc)]
use crate::prelude::QcPreferedIndexing;

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

    /// Context data created by indexing and sorting each user entry.
    pub(crate) data: HashMap<QcDataKey, QcData>,
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

    /// Build an updated [QcContext] with [QcConfig] preferences.
    /// We recommend doing this prior loading any data!
    pub fn with_configuration_preferences(&self, cfg: QcConfig) -> Self {
        let mut s = self.clone();
        s.configuration = cfg;
        s
    }

    /// Returns general [TimeScale] for current [QcContext] and data source
    /// indexed by [QcIndexing] method.
    ///
    /// In case measurements where provided, they will always prevail:
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
    /// context.load_rinex_file("data/NAV/V2/amel0010.21g")
    ///     .unwrap();
    ///
    /// assert_eq!(context.timescale(), Some(TimeScale::GPST));
    /// ```
    ///
    /// SP3 files have unambiguous timescale definition as well.
    /// So they will prevail as long as RINEX measurements were not provided:
    ///
    /// ```
    /// use gnss_qc::prelude::{QcContext, TimeScale};
    ///
    /// // create a new (empty) context
    /// let mut context = QcContext::new();
    ///
    /// // load some data
    /// context.load_gzip_sp3_file("data/SP3/D/COD0MGXFIN_20230500000_01D_05M_ORB.SP3.gz")
    ///     .unwrap();
    ///
    /// assert_eq!(context.timescale(), Some(TimeScale::GPST));
    /// ```
    pub fn product_timescale(
        &self,
        product: QcProductType,
        indexing: QcIndexing,
    ) -> Option<TimeScale> {
        let data = self
            .products_iter(product)
            .filter_map(|(index, v)| if *index == indexing { Some(v) } else { None })
            .reduce(|k, _| k)?;

        if let Some(rinex) = data.inner.as_rinex() {
            match product {
                QcProductType::Observation => {
                    if let Some(header) = rinex.header.obs.as_ref() {
                        if let Some(epoch) = header.timeof_first_obs {
                            return Some(epoch.time_scale);
                        }
                        if let Some(epoch) = header.timeof_last_obs {
                            return Some(epoch.time_scale);
                        }
                    }
                }
                QcProductType::BroadcastNavigation => match rinex.header.constellation {
                    Some(Constellation::Mixed) | None => {}
                    Some(constellation) => {
                        if let Some(timescale) = constellation.timescale() {
                            return Some(timescale);
                        }
                    }
                },
                QcProductType::MeteoObservation => {
                    return Some(TimeScale::UTC);
                }
                QcProductType::PreciseClock => {}
                _ => {}
            }
        }

        #[cfg(feature = "sp3")]
        if let Some(sp3) = data.inner.as_sp3() {
            return Some(sp3.header.timescale);
        }

        None
    }

    /// Returns reference to all inner data matching this [QcProductType].
    pub(crate) fn products_iter(
        &self,
        product: QcProductType,
    ) -> Box<dyn Iterator<Item = (&QcIndexing, &QcData)> + '_> {
        Box::new(self.data.iter().filter_map(move |(key, data)| {
            if key.prod_type == product {
                Some((&key.index, data))
            } else {
                None
            }
        }))
    }

    /// Returns mutable reference to all inner data matching this [QcProductType].
    pub(crate) fn products_iter_mut(
        &mut self,
        product: QcProductType,
    ) -> Box<dyn Iterator<Item = (&QcIndexing, &mut QcData)> + '_> {
        Box::new(self.data.iter_mut().filter_map(move |(key, data)| {
            if key.prod_type == product {
                Some((&key.index, data))
            } else {
                None
            }
        }))
    }

    /// Returns reference to same [QcProductType] and same [QcIndexing] entry
    pub(crate) fn get_product(
        &self,
        product: QcProductType,
        indexing: QcIndexing,
    ) -> Option<&QcData> {
        let matched = self
            .products_iter(product)
            .filter_map(
                |(index, data)| {
                    if *index == indexing {
                        Some(data)
                    } else {
                        None
                    }
                },
            )
            .reduce(|k, _| k)?;

        Some(matched)
    }

    /// Returns mutable reference to same [QcProductType] and same [QcIndexing] entry
    pub(crate) fn get_product_mut(
        &mut self,
        product: QcProductType,
        indexing: QcIndexing,
    ) -> Option<&mut QcData> {
        let matched = self
            .products_iter_mut(product)
            .filter_map(
                |(index, data)| {
                    if *index == indexing {
                        Some(data)
                    } else {
                        None
                    }
                },
            )
            .reduce(|k, _| k)?;

        Some(matched)
    }

    /// True if current [QcContext] is compatible with basic post processed navigation.
    /// It does not mean you can actually perform post processed navigation, you need the "navigation"
    /// feature for that.
    pub fn is_navigation_compatible(&self) -> bool {
        self.has_brdc_navigation() && self.has_observations()
    }

    /// Returns true if provided Input products allow Troposphere bias
    /// model optimization
    pub fn tropo_bias_model_optimization(&self) -> bool {
        self.has_meteo_observations()
    }

    /// Apply preprocessing filter algorithm to mutable [QcContext].
    /// Filter will apply to all internal products when applicable.
    pub fn filter_mut(&mut self, filter: &Filter) {
        for (_, rinex) in self.observations_iter_mut() {
            rinex.filter_mut(filter);
        }

        for (_, rinex) in self.brdc_navigations_iter_mut() {
            rinex.filter_mut(filter);
        }

        for (_, rinex) in self.meteo_observations_iter_mut() {
            rinex.filter_mut(filter);
        }

        for (_, rinex) in self.clocks_rinex_iter_mut() {
            rinex.filter_mut(filter);
        }

        #[cfg(feature = "sp3")]
        for (_, sp3) in self.sp3_agencies_iter_mut() {
            sp3.filter_mut(filter);
        }
    }

    /// Apply desired [Repair]ment to mutable [QcContext].
    /// This only applies to [QcProductType::Observation] products.
    pub fn repair_mut(&mut self, repair: Repair) {
        for (_, rinex) in self.observations_iter_mut() {
            rinex.repair_mut(repair)
        }
    }

    /// Returns True if CPP positioning method
    /// <https://docs.rs/gnss-rtk/latest/gnss_rtk/prelude/enum.Method.html#variant.CodePPP>
    /// may apply to selected data source.
    pub fn is_cpp_navigation_compatible(&self, data_source: &QcIndexing) -> bool {
        if let Some(rinex) = self.get_observation_rinex(data_source) {
            // TODO wrong: only PR
            rinex.carrier_iter().count() > 1
        } else {
            false
        }
    }

    /// Returns True if PPP positioning method
    /// <https://docs.rs/gnss-rtk/latest/gnss_rtk/prelude/enum.Method.html#variant.CodePPP>
    /// may apply to selected data source.
    pub fn is_ppp_navigation_compatible(&self, data_source: &QcIndexing) -> bool {
        if let Some(rinex) = self.get_observation_rinex(data_source) {
            // TODO wrong: only PH+PR
            rinex.carrier_iter().count() > 1
        } else {
            false
        }
    }
}

// impl std::fmt::Debug for QcContext {
//     /// Debug formatting, prints all loaded files per Product category.
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         for product in [
//             QcProductType::Observation,
//             QcProductType::BroadcastNavigation,
//             QcProductType::MeteoObservation,
//             QcProductType::PreciseClock,
//             QcProductType::IONEX,
//             QcProductType::ANTEX,
//             #[cfg(feature = "sp3")]
//             QcProductType::PreciseOrbit,
//         ] {
//             if let Some(files) = self.files(product) {
//                 write!(f, "\n{}: ", product)?;
//                 write!(f, "{:?}", files,)?;
//             }
//         }

//         Ok(())
//     }
// }
