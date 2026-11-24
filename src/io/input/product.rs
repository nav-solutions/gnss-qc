//! Input products: static files loaded into an indexed table, prior processing.
//!
//! Files are classified by [QcIndexing] (agency, receiver, antenna...).
//! Files sharing the same [QcIndexing] and same type are merged together.
use std::{collections::HashMap, path::Path};

use crate::io::input::{
    cfg::QcIndexingConfig, cfg::QcInputConfig, indexing::QcIndexing, QcInputError,
};

use crate::io::{
    rinex::{QcRinexNavSource, QcRinexObsSource},
    source::QcSource,
};

#[cfg(feature = "sp3")]
use crate::io::sp3::QcSp3Source;

use qc_traits::Merge;

use rinex::prelude::{Rinex, RinexType};

#[cfg(feature = "sp3")]
use sp3::prelude::SP3;

#[cfg(feature = "log")]
use log::error;

/// Indexed table of static (file) input products.
/// Each entry is identified by its [QcIndexing]; matching entries are merged.
#[derive(Default, Debug, Clone)]
pub struct QcInputProducts {
    /// Observation RINEX (signal observations), indexed by [QcIndexing].
    pub(crate) obs_rinex: HashMap<QcIndexing, Rinex>,

    /// Navigation RINEX (radio messages), indexed by [QcIndexing].
    pub(crate) nav_rinex: HashMap<QcIndexing, Rinex>,

    /// Meteo RINEX (meteo sensor observations), indexed by [QcIndexing].
    pub(crate) meteo_rinex: HashMap<QcIndexing, Rinex>,

    /// Clock RINEX (precise clock states), indexed by [QcIndexing].
    pub(crate) clk_rinex: HashMap<QcIndexing, Rinex>,

    /// ANTEX: session wide (non temporal) antenna calibration database.
    pub(crate) antex: HashMap<QcIndexing, Rinex>,

    /// High precision orbital states from SP3 files, indexed by [QcIndexing]
    /// (agency by default).
    #[cfg(feature = "sp3")]
    pub(crate) sp3: HashMap<QcIndexing, SP3>,
}

fn file_stem<P: AsRef<Path>>(path: P) -> Result<String, QcInputError> {
    let path = path.as_ref();
    let stem = path.file_stem().ok_or({
        #[cfg(feature = "log")]
        error!("failed to determine {} file stem", path.display());
        QcInputError::FileStemIssue
    })?;

    let stem = stem.to_str().ok_or(QcInputError::FileStemIssue)?;

    // ".crx.gz", ".rnx.gz": strip the remaining extension
    Ok(stem.split('.').next().unwrap_or(stem).to_string())
}

/// Sources and session wide data, obtained from [QcInputProducts].
pub struct QcProductSources {
    /// One source per [QcIndexing] and file type
    pub sources: Vec<Box<dyn QcSource>>,

    /// ANTEX databases
    pub antex: HashMap<QcIndexing, Rinex>,
}

impl QcInputProducts {
    /// Converts this table into engine sources:
    /// - one signal source per Observation RINEX entry
    /// - one ephemeris source per Navigation RINEX entry
    /// - ANTEX entries are session wide data.
    ///
    /// - one precise states source per SP3 entry
    ///
    /// Meteo and Clock RINEX are not converted yet.
    pub fn into_sources(self) -> QcProductSources {
        let mut sources: Vec<Box<dyn QcSource>> = Vec::new();

        let mut obs = self.obs_rinex.into_iter().collect::<Vec<_>>();
        obs.sort_by_key(|a| a.0.to_string());

        for (indexing, rinex) in obs {
            if let Ok(source) = QcRinexObsSource::new(indexing, rinex) {
                sources.push(Box::new(source));
            }
        }

        let mut nav = self.nav_rinex.into_iter().collect::<Vec<_>>();
        nav.sort_by_key(|a| a.0.to_string());

        for (indexing, rinex) in nav {
            if let Ok(source) = QcRinexNavSource::new(indexing, rinex) {
                sources.push(Box::new(source));
            }
        }

        #[cfg(feature = "sp3")]
        {
            let mut sp3 = self.sp3.into_iter().collect::<Vec<_>>();
            sp3.sort_by_key(|(indexing, _)| indexing.to_string());

            for (indexing, sp3) in sp3 {
                sources.push(Box::new(QcSp3Source::new(indexing, sp3)));
            }
        }

        QcProductSources {
            sources,
            antex: self.antex,
        }
    }

    /// Returns true if this table does not contain anything.
    pub fn is_empty(&self) -> bool {
        #[cfg(feature = "sp3")]
        if !self.sp3.is_empty() {
            return false;
        }

        self.obs_rinex.is_empty()
            && self.nav_rinex.is_empty()
            && self.meteo_rinex.is_empty()
            && self.clk_rinex.is_empty()
            && self.antex.is_empty()
    }

    /// Iterates over Observation RINEX entries.
    pub fn obs_rinex_iter(&self) -> impl Iterator<Item = (&QcIndexing, &Rinex)> {
        self.obs_rinex.iter()
    }

    /// Iterates over Navigation RINEX entries.
    pub fn nav_rinex_iter(&self) -> impl Iterator<Item = (&QcIndexing, &Rinex)> {
        self.nav_rinex.iter()
    }

    /// Iterates over ANTEX entries.
    pub fn antex_iter(&self) -> impl Iterator<Item = (&QcIndexing, &Rinex)> {
        self.antex.iter()
    }

    /// Iterates over SP3 entries.
    #[cfg(feature = "sp3")]
    pub fn sp3_iter(&self) -> impl Iterator<Item = (&QcIndexing, &SP3)> {
        self.sp3.iter()
    }

    /// Load local readable file into [QcInputProducts]. Format must be supported.
    /// Every supported format is attempted: prefer [Self::load_rinex] or [Self::load_sp3]
    /// when you know the format.
    pub fn load_file<P: AsRef<Path>>(
        &mut self,
        cfg: &QcInputConfig,
        path: P,
    ) -> Result<(), QcInputError> {
        if let Ok(rinex) = Rinex::from_file(&path) {
            return self.load_rinex(cfg, path, rinex);
        }

        #[cfg(feature = "sp3")]
        if let Ok(sp3) = SP3::from_file(&path) {
            return self.load_sp3(cfg, path, sp3);
        }

        Err(QcInputError::FormatNotSupported)
    }

    /// Load local readable Gzip compressed file into [QcInputProducts].
    pub fn load_gzip_file<P: AsRef<Path>>(
        &mut self,
        cfg: &QcInputConfig,
        path: P,
    ) -> Result<(), QcInputError> {
        if let Ok(rinex) = Rinex::from_gzip_file(&path) {
            return self.load_rinex(cfg, path, rinex);
        }

        #[cfg(feature = "sp3")]
        if let Ok(sp3) = SP3::from_gzip_file(&path) {
            return self.load_sp3(cfg, path, sp3);
        }

        Err(QcInputError::FormatNotSupported)
    }

    /// Load a single [Rinex] into the table.
    /// The [QcIndexing] is determined from user preference (if feasible),
    /// otherwise automatically (infaillible).
    pub fn load_rinex<P: AsRef<Path>>(
        &mut self,
        cfg: &QcInputConfig,
        path: P,
        rinex: Rinex,
    ) -> Result<(), QcInputError> {
        let file_stem = file_stem(path)?;
        let rinex_type = rinex.header.rinex_type;

        let indexing = match cfg.rinex_type_preference(&rinex_type) {
            Some(QcIndexingConfig::Antenna) => rinex
                .header
                .rcvr_antenna
                .as_ref()
                .map(|ant| QcIndexing::Antenna(ant.model.to_string())),
            Some(QcIndexingConfig::Receiver) => rinex
                .header
                .rcvr
                .as_ref()
                .map(|rx| QcIndexing::from_receiver_model_sn(&rx.model, &rx.sn)),
            Some(QcIndexingConfig::Agency) => rinex.header.agency.clone().map(QcIndexing::Agency),
            Some(QcIndexingConfig::Operator) => {
                rinex.header.observer.clone().map(QcIndexing::Operator)
            }
            Some(QcIndexingConfig::Custom(alias)) => Some(QcIndexing::Custom(alias)),
            Some(QcIndexingConfig::FileName) => Some(QcIndexing::Custom(file_stem.clone())),
            None => None,
        };

        let key = indexing.unwrap_or_else(|| QcIndexing::from_rinex_file(&file_stem, &rinex));

        let table = match rinex_type {
            RinexType::ObservationData => &mut self.obs_rinex,
            RinexType::NavigationData => &mut self.nav_rinex,
            RinexType::MeteoData => &mut self.meteo_rinex,
            RinexType::ClockData => &mut self.clk_rinex,
            RinexType::AntennaData => &mut self.antex,
        };

        if let Some(inner) = table.get_mut(&key) {
            inner.merge_mut(&rinex)?;
        } else {
            table.insert(key, rinex);
        }

        Ok(())
    }

    /// Load a single [SP3] into the table.
    /// The [QcIndexing] is determined from user preference ([QcInputConfig::sp3_indexing]),
    /// otherwise automatically: agency, file name when the agency is not described.
    #[cfg(feature = "sp3")]
    pub fn load_sp3<P: AsRef<Path>>(
        &mut self,
        cfg: &QcInputConfig,
        path: P,
        sp3: SP3,
    ) -> Result<(), QcInputError> {
        let file_stem = file_stem(path)?;
        let agency = sp3.header.agency.trim();

        let key = match &cfg.sp3_indexing {
            Some(QcIndexingConfig::Custom(alias)) => QcIndexing::Custom(alias.clone()),
            Some(QcIndexingConfig::FileName) => QcIndexing::Custom(file_stem),
            // agency (preferred), or preferences that do not apply to SP3 (receiver,
            // antenna, operator): automatic indexing
            _ => {
                if agency.is_empty() {
                    QcIndexing::Custom(file_stem)
                } else {
                    QcIndexing::Agency(agency.to_string())
                }
            }
        };

        if let Some(inner) = self.sp3.get_mut(&key) {
            Merge::merge_mut(inner, &sp3).map_err(|e| QcInputError::Sp3Merge(e.to_string()))?;
        } else {
            self.sp3.insert(key, sp3);
        }

        Ok(())
    }
}
