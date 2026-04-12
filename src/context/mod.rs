//! GNSS dataset definition.
mod preferences;

#[cfg(not(feature = "navigation")]
mod default;

use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use crate::prelude::{Rinex, TimeScale};

// use qc_traits::Merge;

// pub(crate) mod blob;
// use blob::BlobData;

// pub use preferences::QcDatasetPreferences;

// pub(crate) mod input;

// #[cfg(feature = "flate2")]
// #[cfg_attr(docsrs, doc(cfg(feature = "flate2")))]
// mod flate2;

// #[cfg(feature = "sp3")]
// #[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
// mod sp3;

// #[cfg(feature = "navigation")]
// #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
// mod navigation;

// #[cfg(feature = "navigation")]
// #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
// pub mod time;

use qc_traits::{Filter, Preprocessing, Repair, RepairTrait};

//use crate::{
//    error::Error, 
//    prelude::ProductType,
//};

#[cfg(feature = "navigation")]
use crate::prelude::{Almanac, Frame};

/// [QcContext] is a general structure capable to store most common
/// GNSS data. It is dedicated to post processing workflows,
/// precise timing or atmosphere analysis.
///
/// It conveniently implements [PartialEq] and [Hash] for quick session
/// comparisons.
#[derive(Clone, PartialEq, Eq)]
pub struct QcContext {
    // /// Provided [InputProducts]
    // input: input::InputProducts,
    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    /// Latest [Almanac]
    almanac: Almanac,

    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    /// ECEF [Frame]
    earth_cef: Frame,

    // /// [QcContextPreferences]
    // pub preferences: QcContextPreferences,

    /// Contextualized dataset, now read-only
    dataset: Arc<Dataset>,
}

impl QcContext {

    // /// Returns reference to [QcInputProducts].
    // fn input(&self) -> &QcInputProducts {
    //     self.input
    // }

    // /// Returns name of this context.
    // /// Context is named after the file considered as Primary, see [Self::primary_path].
    // /// If no files were previously loaded, simply returns "Undefined".
    // pub fn name(&self) -> String {
    //     if let Some(path) = self.primary_path() {
    //         path.file_name()
    //             .unwrap_or(OsStr::new("Undefined"))
    //             .to_string_lossy()
    //             // removes possible .crx ; .gz extensions
    //             .split('.')
    //             .next()
    //             .unwrap_or("Undefined")
    //             .to_string()
    //     } else {
    //         "Undefined".to_string()
    //     }
    // }

    // /// Returns reference to inner data of given category
    // pub(crate) fn data(&self, product: ProductType) -> Option<&BlobData> {
    //     self.blob
    //         .iter()
    //         .filter_map(|(prod_type, data)| {
    //             if *prod_type == product {
    //                 Some(data)
    //             } else {
    //                 None
    //             }
    //         })
    //         .reduce(|k, _| k)
    // }

    // /// Returns mutable reference to inner data of given category
    // pub(crate) fn data_mut(&mut self, product: ProductType) -> Option<&mut BlobData> {
    //     self.blob
    //         .iter_mut()
    //         .filter_map(|(prod_type, data)| {
    //             if *prod_type == product {
    //                 Some(data)
    //             } else {
    //                 None
    //             }
    //         })
    //         .reduce(move |k, _| k)
    // }

    // /// True if current [QcContext] is compatible with basic post processed navigation.
    // /// It does not mean you can actually perform post processed navigation, you need the "navigation"
    // /// feature for that.
    // pub fn is_navigation_compatible(&self) -> bool {
    //     self.observation().is_some() && self.brdc_navigation().is_some()
    // }

    // /// Returns true if provided Input products allow Ionosphere bias
    // /// model optimization
    // pub fn iono_bias_model_optimization(&self) -> bool {
    //     // self.ionex().is_some() // TODO: BRDC V3 or V4
    //     false
    // }

    // /// Returns true if provided Input products allow Troposphere bias
    // /// model optimization
    // pub fn tropo_bias_model_optimization(&self) -> bool {
    //     self.has_meteo()
    // }

    // /// Apply preprocessing filter algorithm to mutable [Self].
    // /// Filter will apply to all data contained in the context.
    // pub fn filter_mut(&mut self, filter: &Filter) {
    //     if let Some(data) = self.observation_mut() {
    //         data.filter_mut(filter);
    //     }
    //     if let Some(data) = self.brdc_navigation_mut() {
    //         data.filter_mut(filter);
    //     }
    //     // if let Some(data) = self.doris_mut() {
    //     //    data.filter_mut(filter);
    //     //}
    //     if let Some(data) = self.meteo_mut() {
    //         data.filter_mut(filter);
    //     }
    //     if let Some(data) = self.clock_mut() {
    //         data.filter_mut(filter);
    //     }

    //     // if let Some(data) = self.ionex_mut() {
    //     //     data.filter_mut(filter);
    //     // }

    //     #[cfg(feature = "sp3")]
    //     if let Some(data) = self.sp3_mut() {
    //         data.filter_mut(filter);
    //     }
    // }

    // /// Fix given [Repair] condition
    // pub fn repair_mut(&mut self, r: Repair) {
    //     if let Some(rinex) = self.observation_mut() {
    //         rinex.repair_mut(r);
    //     }
    // }

    // /// True if current [QcContext] is compatible with CPP positioning method
    // /// <https://docs.rs/gnss-rtk/latest/gnss_rtk/prelude/enum.Method.html#variant.CodePPP>.
    // /// This does not mean you can deploy a navigation solver, because that requires
    // /// the "navigation" create feature.
    // pub fn is_cpp_navigation_compatible(&self) -> bool {
    //     // TODO: improve: only PR
    //     if let Some(obs) = self.observation() {
    //         obs.carrier_iter().count() > 1
    //     } else {
    //         false
    //     }
    // }

    // /// Returns True if current [QcContext] is compatible with PPP positioning method
    // /// <https://docs.rs/gnss-rtk/latest/gnss_rtk/prelude/enum.Method.html#variant.PPP>.
    // /// This does not mean you can deploy a navigation solver, because that requires
    // /// the "navigation" create feature.
    // pub fn is_ppp_navigation_compatible(&self) -> bool {
    //     // TODO: check PH as well
    //     self.is_cpp_navigation_compatible()
    // }

    // #[cfg(not(feature = "sp3"))]
    // /// SP3 is required for 100% PPP compatibility
    // pub fn is_ppp_ultra_navigation_compatible(&self) -> bool {
    //     false
    // }
}

// impl std::fmt::Debug for QcContext {
//     /// Debug formatting, prints all loaded files per Product category.
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "Primary: \"{}\"", self.name())?;
//         for product in [
//             ProductType::Observation,
//             ProductType::BroadcastNavigation,
//             ProductType::MeteoObservation,
//             ProductType::HighPrecisionClock,
//             // ProductType::IONEX,
//             ProductType::ANTEX,
//             #[cfg(feature = "sp3")]
//             ProductType::HighPrecisionOrbit,
//         ] {
//             if let Some(files) = self.files(product) {
//                 write!(f, "\n{}: ", product)?;
//                 write!(f, "{:?}", files,)?;
//             }
//         }
//         Ok(())
//     }
// }
