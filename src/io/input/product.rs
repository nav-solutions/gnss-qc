//! Input product
use std::{collections::HashMap, path::Path};

use crate::io::input::{
    cfg::QcIndexingConfig,
    cfg::QcInputConfig,
    indexing::{QcIndexing, QcIndexingError},
    key::QcProductKey,
    QcInputError,
};

use qc_traits::Merge;

use rinex::prelude::{Rinex, RinexType};

#[cfg(feature = "sp3")]
use sp3::prelude::SP3;

// impl QcInputProduct {
//     /// Returns reference to underlying [Rinex] when that applies
//     pub fn as_rinex(&self) -> Option<&Rinex> {
//         match self {
//             Self::RINEX(rinex) => Some(rinex),
//             _ => None,
//         }
//     }
//
//     /// Returns mutable reference to underlying [Rinex] when that applies
//     pub fn as_rinex_mut(&mut self) -> Option<&mut Rinex> {
//         match self {
//             Self::RINEX(rinex) => Some(rinex),
//             _ => None,
//         }
//     }
//
//     /// Returns reference to underlying [SP3] when that applies
//     #[cfg(feature = "sp3")]
//     pub fn as_sp3(&self) -> Option<&SP3> {
//         match self {
//             Self::SP3(sp3) => Some(sp3),
//             _ => None,
//         }
//     }
//
//     /// Returns mutable reference to underlying [SP3] when that applies
//     #[cfg(feature = "sp3")]
//     pub fn as_sp3_mut(&mut self) -> Option<&mut SP3> {
//         match self {
//             Self::SP3(sp3) => Some(sp3),
//             _ => None,
//         }
//     }
// }

/// Structure that holds all supported input products that we can
/// then serialize and process.
pub struct QcInputProducts {
    /// Observation RINEX, indexed by [QcIndexing].
    /// Matching [QcIndexing] are merged together into a single [Rinex] structure that
    /// we can then serialize & process.
    obs_rinex: HashMap<QcIndexing, Rinex>,

    /// Navigation RINEX, indexed by [QcIndexing].
    /// Matching [QcIndexing] are merged together into a single [Rinex] structure that
    /// we can then serialize & process.
    nav_rinex: HashMap<QcIndexing, Rinex>,
}

impl QcInputProducts {
    /// Load local readable [Path] into [QcInputProducts] database, ready to be processed.
    /// File format must be supported.
    pub fn load_file<P: AsRef<Path>>(
        &mut self,
        cfg: QcInputConfig,
        path: P,
    ) -> Result<(), QcInputError> {
        if let Ok(rinex) = Rinex::from_file(path) {
            Self::load_rinex(cfg, path)?;
            Ok(())
        } else {
            Err(QcInputError::FileNotSupported(""))
        }
    }

    /// Load single [Rinex] file into input products.
    /// ## Input
    /// - indexing: [QcIndexing] preference (if any); otherwise this is deduced from dataset
    /// automatically
    /// - rinex: input [Rinex]
    pub fn load_rinex(&mut self, cfg: QcInputConfig, rinex: Rinex) -> Result<(), QcInputError> {
        let prefered = match rinex.header.rinex_type {
            RinexType::ObservationData => {}
            RinexType::NavigationData => {}
            Some(preferences) => preferences,
            None => QcIndexingConfig::from_rinex_type(rinex.header.rinex_type),
        };

        // tries matching preferences
        let indexing = match prefered {
            QcIndexingConfig::Antenna => match rinex.header.rcvr_antenna {
                Some(ant) => Some(QcIndexing::Antenna(ant.model.to_string())),
                None => None,
            },
            QcIndexingConfig::Receiver => match rinex.header.rcvr {
                Some(rx) => Some(QcIndexing::Receiver(rx.model.to_string())),
                None => None,
            },
            QcIndexingConfig::Agency => match rinex.header.agency {
                Some(agency) => Some(QcIndexing::Agency(agency.clone())),
                None => None,
            },
            QcIndexingConfig::Custom(value) => {
                // always applies
                Some(QcIndexing::Custom(value))
            }
        };

        let key = match indexing {
            Some(matched) => {
                // preferences have been met
                matched
            }
            None => {
                // automatically deduce, by order of internal preferences
                QcIndexing::from_rinex_file(&rinex)
            }
        };

        if let Some(inner) = self.obs_rinex.get_mut(&key) {
            inner.merge_mut(&rinex)?;
        } else {
            // new table entry
            self.obs_rinex.insert(key, rinex);
        }

        Ok(())
    }
}
