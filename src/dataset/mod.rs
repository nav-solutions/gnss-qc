//! GNSS dataset definition.
mod products;
mod source;

use std::{collections::HashMap, fs::Path};

use walkdir::WalkDir;

pub use products::QcInputProduct;
pub use source::QcInputSource;

use rinex::prelude::Rinex;

#[cfg(feature = "sp3")]
use sp3::prelude::SP3;

#[derive(Clone)]
pub enum QcData {
    RINEX(Rinex),

    #[cfg(feature = "sp3")]
    SP3(SP3),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct QcDataIndex {
    /// [QcInputProduct]
    pub product: QcInputProduct,

    /// [QcInputSource]
    pub source: QcInputSource,
}

/// [QcDataset] wrapps all static input products that
/// we index with [QcDataIndex] and gathered from local files.
#[derive(Default, Clone)]
pub struct QcDataset {
    /// Data is indexed by [QcDataIndex] and stored as [QcData].
    data: HashMap<QcDataIndex, QcData>,
}

impl QcDataset {
    #[cfg(not(feature = "navigation"))]
    pub fn contextualization(&self, preferences: Option<QcContextPreferences>) -> QcContext {
        let user_preferences = preferences.unwrap_or_default();
        QcContext {
            user_preferences,
            data: Arc::new(self.data),
        }
    }

    #[cfg(feature = "navigation")]
    pub fn contextualization(
        &self,
        preferences: Option<QcContextPreferences>,
    ) -> Result<QcContext, Error> {
        let user_preferences = preferences.unwrap_or_default();

        // post-processed navigation will require the definition
        // of an Almanac and a reference Frame. We definte it once & for all.
        #[cfg(feature = "navigation")]
        let (almanac, earth_cef) = user_preferences.to_almanac_frame()?;

        QcContext {
            user_preferences,

            #[cfg(feature = "navigation")]
            almanac,

            #[cfg(feature = "navigation")]
            earth_cef,

            data: Arc::new(self.data),
        }
    }
}
