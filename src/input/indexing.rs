#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use hifitime::Epoch;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum QcIndexing {
    /// Indexed by antenna model or alias. Used for signal sources
    Antenna(String),

    /// Indexed by GNSS receivered. Used for signal sources.
    Receiver(String),

    /// Indexed by operator (name or alias), sometimes refered to as
    /// "observer" in RINEX terminology.
    Operator(String),

    /// Indexed by agency name or alias (data publisher). Used for high
    /// precision products.
    Agency(String),

    /// Indexed by custom alias
    Custom(String),
}

impl QcIndexing {
    /// Defines a new [QcIndexing::Antenna] classification for this model or equipment.
    pub fn antenna_indexing(antenna: &str) -> Self {
        Self::Antenna(antenna.to_string())
    }

    /// Defines a new [QcIndexing::Receiver] classification for this model or equipment.
    pub fn receiver_indexing(gnss_receiver: &str) -> Self {
        Self::Receiver(gnss_receiver.to_string())
    }

    /// Defines a new [QcIndexing::Agency] classification for this model or equipment.
    pub fn agency_indexing(agency: &str) -> Self {
        Self::Agency(agency.to_string())
    }

    /// Defines a new [QcIndexing::Custom] classification alias.
    pub fn custom_indexing(alias: &str) -> Self {
        Self::Custom(alias.to_string())
    }

    /// Defines a new [QcIndexing::Operator] classification.
    pub fn operator_indexing(name: &str) -> Self {
        Self::Operator(name.to_string())
    }
}

#[cfg(test)]
mod test {
    use crate::input::indexing::QcIndexing;

    #[test]
    fn test_antenna_indexing() {
        let test = QcIndexing::antenna_indexing("super-model");
        assert_eq!(test, QcIndexing::Antenna("super-model".to_string()));
    }

    #[test]
    fn test_receiver_indexing() {
        let test = QcIndexing::receiver_indexing("super-model");
        assert_eq!(test, QcIndexing::Receiver("super-model".to_string()));
    }

    #[test]
    fn test_operator_indexing() {
        let test = QcIndexing::operator_indexing("myself");
        assert_eq!(test, QcIndexing::Operator("myself".to_string()));
    }

    #[test]
    fn test_agency_indexing() {
        let test = QcIndexing::agency_indexing("lab");
        assert_eq!(test, QcIndexing::Agency("lab".to_string()));
    }

    #[test]
    fn test_custom_indexing() {
        let test = QcIndexing::custom_indexing("alias");
        assert_eq!(test, QcIndexing::Custom("alias".to_string()));
    }
}
