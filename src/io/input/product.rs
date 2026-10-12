//! Input product
use std::{collections::HashMap, path::Path};

use crate::io::input::{
    cfg::QcIndexingConfig, cfg::QcInputConfig, indexing::QcIndexing, QcInputError,
};

use qc_traits::Merge;

use rinex::prelude::Rinex;

#[cfg(feature = "sp3")]
use sp3::prelude::SP3;

#[cfg(feature = "ionex")]
use sp3::prelude::IONEX;

#[cfg(feature = "log")]
use log::error;

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

    /// High Precision orbital states from static SP3 files.
    /// Indexed by [QcIndexing].
    /// Matching laboratories (data producers) are merged together into a single [SP3] structure
    /// that we can then serialize & process.
    #[cfg(feature = "sp3")]
    sp3: HashMap<QcIndexing, SP3>,
}

impl QcInputProducts {
    /// Load local readable [Path] into [QcInputProducts] database, ready to be processed.
    /// File format must be supported.
    /// For better effiency, select the correct format directly and avoid guessing.
    pub fn load_file<P: AsRef<Path>>(
        &mut self,
        cfg: QcInputConfig,
        path: P,
    ) -> Result<(), QcInputError> {
        if let Ok(rinex) = Rinex::from_file(&path) {
            self.load_rinex(cfg, path, rinex)?;
            return Ok(());
        }

        #[cfg(feature = "sp3")]
        {
            if let Ok(sp3) = SP3::from_file(&path) {
                self.load_sp3(cfg, path, sp3)?;
                return Ok(());
            }
        }

        #[cfg(feature = "ionex")]
        {
            if let Ok(ionex) = IONEX::from_file(&path) {
                self.load_ionex(cfg, path, ionex)?;
                return Ok(());
            }
        }

        Err(QcInputError::FormatNotSupported)
    }

    /// Local local readable Gzip compressed [Path] into [QcInputProducts] database, ready to be
    /// processed. File format must be supported. The library will try every single format
    /// supported. For better effiency, select the correct format directly and avoid guessing.
    #[cfg(feature = "flate2")]
    pub fn load_gzip_file<P: AsRef<Path>>(
        &mut self,
        cfg: QcInputConfig,
        path: P,
    ) -> Result<(), QcInputError> {
        if let Ok(rinex) = Rinex::from_gzip_file(&path) {
            self.load_rinex(cfg, path, rinex)?;
            return Ok(());
        }

        #[cfg(feature = "sp3")]
        {
            if let Ok(sp3) = SP3::from_gzip_file(&path) {
                self.load_sp3(cfg, path, sp3)?;
                return Ok(());
            }
        }

        #[cfg(feature = "ionex")]
        {
            if let Ok(ionex) = IONEX::from_gzip_file(&path) {
                self.load_ionex(cfg, path, ionex)?;
                return Ok(());
            }
        }

        Err(QcInputError::FormatNotSupported)
    }

    /// Load single [Rinex] file into input products.
    /// ## Input
    /// - indexing: [QcIndexing] preference (if any); otherwise this is deduced from dataset
    /// automatically
    /// - rinex: input [Rinex]
    pub fn load_rinex<P: AsRef<Path>>(
        &mut self,
        cfg: QcInputConfig,
        path: P,
        rinex: Rinex,
    ) -> Result<(), QcInputError> {
        let path = path.as_ref();

        let file_stem = path
            .file_stem()
            .ok_or({
                error!("failed to determine {} file stem", path.display());
                QcInputError::FileStemIssue
            })?
            .to_str()
            .ok_or(QcInputError::FileStemIssue)?;

        let prefered = cfg.rinex_type_preference(&rinex.header.rinex_type);

        // tries matching preferences
        let indexing = match prefered {
            Some(QcIndexingConfig::Antenna) => match &rinex.header.rcvr_antenna {
                Some(ant) => Some(QcIndexing::Antenna(ant.model.to_string())),
                None => None,
            },
            Some(QcIndexingConfig::Receiver) => match &rinex.header.rcvr {
                Some(rx) => Some(QcIndexing::Receiver(rx.model.to_string())),
                None => None,
            },
            Some(QcIndexingConfig::Agency) => match &rinex.header.agency {
                Some(agency) => Some(QcIndexing::Agency(agency.clone())),
                None => None,
            },
            Some(QcIndexingConfig::Operator) => match &rinex.header.observer {
                Some(observer) => Some(QcIndexing::Operator(observer.clone())),
                None => None,
            },
            Some(QcIndexingConfig::Custom(alias)) => Some(QcIndexing::Custom(alias)),
            None => None,
        };

        let key = match indexing {
            Some(indexing) => indexing,
            None => {
                // no preferences for this format
                // => select classification method internally (infaillible)
                QcIndexing::from_rinex_file(file_stem, &rinex)
            },
        };

        if let Some(inner) = self.obs_rinex.get_mut(&key) {
            inner.merge_mut(&rinex)?;
        } else {
            // new table entry
            self.obs_rinex.insert(key, rinex);
        }

        Ok(())
    }

    /// Load single [SP3] file into input products.
    /// ## Input
    /// - indexing: [QcIndexing] preference (if any); otherwise this is deduced from dataset
    /// automatically
    /// - rinex: input [Rinex]
    #[cfg(feature = "sp3")]
    pub fn load_sp3<P: AsRef<Path>>(&mut self, path: P, sp3: SP3) -> Result<(), QcInputError> {
        let path = path.as_ref();

        let file_stem = path
            .file_stem()
            .ok_or({
                error!("failed to determine {} file stem", path.display());
                QcInputError::FileStemIssue
            })?
            .to_str()
            .ok_or(QcInputError::FileStemIssue)?;

        // SP3 files: always indexed by data producer
        let key = QcIndexing::Agency(sp3.header.agency);

        if let Some(inner) = self.sp3.get_mut(&key) {
            inner.merge_mut(&sp3)?;
        } else {
            // new table entry
            self.sp3.insert(key, sp3);
        }

        Ok(())
    }

    /// Load single [IONEX] file into input products.
    /// ## Input
    /// - indexing: [QcIndexing] preference (if any); otherwise this is deduced from dataset
    /// automatically
    /// - ionex: input [IONEX]
    #[cfg(feature = "sp3")]
    pub fn load_ionex<P: AsRef<Path>>(
        &mut self,
        cfg: QcInputConfig,
        path: P,
        ionex: IONEX,
    ) -> Result<(), QcInputError> {
        let path = path.as_ref();

        let file_stem = path
            .file_stem()
            .ok_or({
                error!("failed to determine {} file stem", path.display());
                QcInputError::FileStemIssue
            })?
            .to_str()
            .ok_or(QcInputError::FileStemIssue)?;

        // IONEX files: always indexed by data producer
        let key = QcIndexing::Agency(sp3.header.agency);

        if let Some(inner) = self.ionex.get_mut(&key) {
            inner.merge_mut(&ionex)?;
        } else {
            // new table entry
            self.sp3.insert(key, ionex);
        }

        Ok(())
    }
}
