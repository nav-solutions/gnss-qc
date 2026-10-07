use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use hifitime::Epoch;

use rinex::prelude::{Rinex, RinexType};

/// Errors that may arise during the process of input products indexing.
/// This only applies to input products (local files) loading into the local database
/// prior processing. For input streams (real time slots, mount point etc..) their nature
/// is already known.
#[derive(Debug, Error)]
pub enum QcIndexingError {
    /// When loading a static file into the Qc product database,
    /// and no [QcIndexing] classfication method is set (by user preferences),
    /// the library will attempt to automatically select a [QcIndexing] method.
    /// This should be an infaillible process. Our fallback method is to simply
    /// use the local filename as a unique entry. If the OS file name determination method
    /// fails, we will wind up here.
    #[error("failed to determine file meta data")]
    FileMeta,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

    /// Select a [QcIndexing] method for this [Rinex] file (infaillible)
    pub fn from_rinex_file(file_stem: &str, rinex: &Rinex) -> Self {
        match rinex.header.rinex_type {
            RinexType::ObservationData => {
                // 1. prefered is Receiver model
                match &rinex.header.rcvr {
                    Some(rx) => Self::Receiver(rx.model.to_string()),
                    None => {
                        // 2. Antenna model comes 2nd
                        match &rinex.header.rcvr_antenna {
                            Some(ant) => Self::Antenna(ant.model.to_string()),
                            None => {
                                // 3. Operator name 3rd
                                match &rinex.header.observer {
                                    Some(observer) => Self::Operator(observer.to_string()),
                                    None => {
                                        // 4. File stem by default
                                        Self::Custom(file_stem.to_string())
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                // prefered is agency
                match &rinex.header.agency {
                    Some(agency) => Self::Agency(agency.clone()),
                    None => Self::Custom(file_stem.to_string()),
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::io::input::indexing::QcIndexing;

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
