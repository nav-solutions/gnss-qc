//! Input user preferences
use rinex::prelude::RinexType;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Indexing preference configuration preset.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum QcIndexingConfig {
    /// Indexed by antenna model or alias. Used for signal sources
    Antenna,

    /// Indexed by GNSS receivered. Used for signal sources.
    Receiver,

    /// Indexed by operator (name or alias), sometimes refered to as
    /// "observer" in RINEX terminology.
    Operator,

    /// Agency or data provider (laboratory) name
    Agency,

    /// Custom name
    Custom(String),

    /// File name (without extensions): each file is a distinct product, unless
    /// several files share the same name.
    FileName,
}

impl QcIndexingConfig {
    /// Returns default [QcIndexingConfig] preference for given [RinexType]
    pub fn from_rinex_type(rtype: RinexType) -> Self {
        match rtype {
            RinexType::ObservationData => Self::Receiver,
            _ => Self::Agency,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct QcInputConfig {
    /// Prefered Indexing method for signal streams.
    /// When set to none, the library will index automatically based on the input stream.
    pub obs_rinex_indexing: Option<QcIndexingConfig>,

    /// Prefered Indexing method for ephemeris streams.
    /// When set to none, the library will index automatically based on the input stream.
    pub nav_rinex_indexing: Option<QcIndexingConfig>,

    /// Prefered Indexing method for Meteo RIENX data (meteo sensor observations)
    pub meteo_rinex_indexing: Option<QcIndexingConfig>,

    /// Prefered Indexing method for RINEX Clock data (ground or spaceborn clocks description).
    pub clk_rinex_indexing: Option<QcIndexingConfig>,

    /// Prefered Indexing method for SP3 files (precise products).
    /// [QcIndexingConfig::Agency] (default), [QcIndexingConfig::Custom] and
    /// [QcIndexingConfig::FileName] apply; SP3 files do not describe receivers,
    /// antennas or operators: these preferences fall back to automatic indexing.
    pub sp3_indexing: Option<QcIndexingConfig>,
}

impl QcInputConfig {
    /// Returns [QcIndexingConfig] preference for given Rinex format.
    pub fn rinex_type_preference(&self, format: &RinexType) -> Option<QcIndexingConfig> {
        match format {
            RinexType::ObservationData => self.obs_rinex_indexing.clone(),
            RinexType::NavigationData => self.nav_rinex_indexing.clone(),
            RinexType::MeteoData => self.meteo_rinex_indexing.clone(),
            RinexType::ClockData => self.clk_rinex_indexing.clone(),
            // ANTEX: session wide database, indexed by file name
            RinexType::AntennaData => None,
        }
    }
}

impl Default for QcInputConfig {
    /// Creates the default (prefered) [QcInputConfig]uration:
    /// - signals indexed by [QcIndexing::Receiver] GNSS receiver sampler
    /// - ephemeris messages, meteo and clock RINEX, SP3 indexed by [QcIndexing::Agency]
    ///
    /// [QcIndexing]: crate::prelude::QcIndexing
    /// [QcIndexing::Receiver]: crate::prelude::QcIndexing::Receiver
    /// [QcIndexing::Agency]: crate::prelude::QcIndexing::Agency
    fn default() -> Self {
        Self {
            nav_rinex_indexing: Some(QcIndexingConfig::Agency),
            obs_rinex_indexing: Some(QcIndexingConfig::Receiver),
            meteo_rinex_indexing: Some(QcIndexingConfig::Agency),
            clk_rinex_indexing: Some(QcIndexingConfig::Agency),
            sp3_indexing: Some(QcIndexingConfig::Agency),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::io::input::cfg::{QcIndexingConfig, QcInputConfig};

    #[test]
    fn test_default() {
        let default = QcInputConfig::default();
        assert_eq!(default.nav_rinex_indexing, Some(QcIndexingConfig::Agency));
        assert_eq!(default.obs_rinex_indexing, Some(QcIndexingConfig::Receiver));
        assert_eq!(default.sp3_indexing, Some(QcIndexingConfig::Agency));
    }
}
