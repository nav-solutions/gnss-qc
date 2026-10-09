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
}

impl QcIndexingConfig {
    /// Returns [QcIndexingConfig] preference for given Rinex format.
    pub fn rinex_type_preference(&self, format: &RinexType) -> Option<QcIndexingConfig> {
        match format {
            RinexType::ObservationData => self.obs_rinex_indexing,
            RinexType::NavigationData => self.nav_rinex_indexing,
            RinexType::MeteoData => self.meteo_rinex_indexing,
            RinexType::ClockData => self.clk_rinex_indexing,
        }
    }
}

impl Default for QcInputConfig {
    /// Creates the default (prefered) [QcInputConfig]uration:
    /// - signals indexed by [QcIndexing::Receiver] GNSS receiver sampler
    /// - ephemeris messages indexed by
    fn default() -> Self {
        Self {
            nav_rinex_indexing: Some(QcIndexingConfig::Agency),
            obs_rinex_indexing: Some(QcIndexingConfig::Receiver),
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
    }
}
