//! Input user preferences

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
    Custom,
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
    use crate::input::cfg::{QcIndexingConfig, QcInputConfig};

    #[test]
    fn test_default() {
        let default = QcInputConfig::default();
        assert_eq!(default.nav_rinex_indexing, Some(QcIndexingConfig::Agency));
        assert_eq!(default.obs_rinex_indexing, Some(QcIndexingConfig::Receiver));
    }
}
