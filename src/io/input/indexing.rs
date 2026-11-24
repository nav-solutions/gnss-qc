use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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

    /// [QcIndexing] parsing: the name is empty
    #[error("empty indexing name")]
    EmptyName,
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

/// Prefixes of the textual representation, see [QcIndexing::from_str].
const ANTENNA_PREFIX: &str = "antenna:";
const RECEIVER_PREFIX: &str = "receiver:";
const OPERATOR_PREFIX: &str = "operator:";
const AGENCY_PREFIX: &str = "agency:";
const CUSTOM_PREFIX: &str = "custom:";

const PREFIXES: [&str; 5] = [
    ANTENNA_PREFIX,
    RECEIVER_PREFIX,
    OPERATOR_PREFIX,
    AGENCY_PREFIX,
    CUSTOM_PREFIX,
];

impl std::fmt::Display for QcIndexing {
    /// Textual representation: `antenna:NAME`, `receiver:NAME`, `operator:NAME`,
    /// `agency:NAME`, and `NAME` for custom aliases. A custom alias that starts with
    /// one of these prefixes is written `custom:NAME`, so that
    /// [QcIndexing::from_str] always reverses this representation.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Antenna(name) => write!(f, "{}{}", ANTENNA_PREFIX, name),
            Self::Receiver(name) => write!(f, "{}{}", RECEIVER_PREFIX, name),
            Self::Operator(name) => write!(f, "{}{}", OPERATOR_PREFIX, name),
            Self::Agency(name) => write!(f, "{}{}", AGENCY_PREFIX, name),
            Self::Custom(name) => {
                if PREFIXES.iter().any(|prefix| name.starts_with(prefix)) {
                    write!(f, "{}{}", CUSTOM_PREFIX, name)
                } else {
                    write!(f, "{}", name)
                }
            }
        }
    }
}

impl std::str::FromStr for QcIndexing {
    type Err = QcIndexingError;

    /// Parses the textual representation of [QcIndexing] (reciprocal of
    /// [std::fmt::Display]):
    /// - `antenna:NAME` → [QcIndexing::Antenna]
    /// - `receiver:NAME` → [QcIndexing::Receiver]
    /// - `operator:NAME` → [QcIndexing::Operator]
    /// - `agency:NAME` → [QcIndexing::Agency]
    /// - `custom:NAME` or any other `NAME` → [QcIndexing::Custom]
    ///
    /// Prefixes are case sensitive, names are kept as is (spaces included).
    /// An empty name is an error.
    ///
    /// ```
    /// use std::str::FromStr;
    /// use gnss_qc::prelude::QcIndexing;
    ///
    /// let receiver = QcIndexing::from_str("receiver:SEPT POLARX5 #3047937").unwrap();
    /// assert_eq!(receiver, QcIndexing::Receiver("SEPT POLARX5 #3047937".to_string()));
    /// assert_eq!(receiver.to_string(), "receiver:SEPT POLARX5 #3047937");
    ///
    /// let alias = QcIndexing::from_str("ESBC00DNK_R_20201770000_01D_MN").unwrap();
    /// assert_eq!(alias, QcIndexing::Custom("ESBC00DNK_R_20201770000_01D_MN".to_string()));
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (variant, name): (fn(String) -> Self, &str) =
            if let Some(name) = s.strip_prefix(ANTENNA_PREFIX) {
                (Self::Antenna, name)
            } else if let Some(name) = s.strip_prefix(RECEIVER_PREFIX) {
                (Self::Receiver, name)
            } else if let Some(name) = s.strip_prefix(OPERATOR_PREFIX) {
                (Self::Operator, name)
            } else if let Some(name) = s.strip_prefix(AGENCY_PREFIX) {
                (Self::Agency, name)
            } else if let Some(name) = s.strip_prefix(CUSTOM_PREFIX) {
                (Self::Custom, name)
            } else {
                (Self::Custom, s)
            };

        if name.is_empty() {
            return Err(QcIndexingError::EmptyName);
        }

        Ok(variant(name.to_string()))
    }
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

    /// Defines a [QcIndexing::Receiver] from receiver model and serial number.
    /// The serial number is required to distinguish two identical receivers
    /// (typically, an RTK base and rover of the same model).
    pub fn from_receiver_model_sn(model: &str, sn: &str) -> Self {
        let (model, sn) = (model.trim(), sn.trim());
        if sn.is_empty() {
            Self::Receiver(model.to_string())
        } else {
            Self::Receiver(format!("{} #{}", model, sn))
        }
    }

    /// Select a [QcIndexing] method for this [Rinex] file (infaillible)
    pub fn from_rinex_file(file_stem: &str, rinex: &Rinex) -> Self {
        match rinex.header.rinex_type {
            RinexType::ObservationData => {
                // 1. prefered is Receiver model
                match &rinex.header.rcvr {
                    Some(rx) => Self::from_receiver_model_sn(&rx.model, &rx.sn),
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
    use std::str::FromStr;

    use crate::io::input::indexing::{QcIndexing, QcIndexingError};

    /// Display then FromStr gives back the same value
    fn assert_round_trip(indexing: QcIndexing, expected: &str) {
        let text = indexing.to_string();
        assert_eq!(text, expected);
        assert_eq!(QcIndexing::from_str(&text).unwrap(), indexing, "{}", text);
    }

    #[test]
    fn antenna_from_str() {
        assert_round_trip(
            QcIndexing::Antenna("TRM59800.00 SCIS".to_string()),
            "antenna:TRM59800.00 SCIS",
        );
    }

    #[test]
    fn receiver_from_str() {
        assert_round_trip(
            QcIndexing::Receiver("SEPT POLARX5 #3047937".to_string()),
            "receiver:SEPT POLARX5 #3047937",
        );
    }

    #[test]
    fn operator_from_str() {
        assert_round_trip(QcIndexing::Operator("SDFE".to_string()), "operator:SDFE");
    }

    #[test]
    fn agency_from_str() {
        assert_round_trip(QcIndexing::Agency("GRGS".to_string()), "agency:GRGS");
    }

    #[test]
    fn custom_from_str() {
        assert_round_trip(
            QcIndexing::Custom("ESBC00DNK_R_20201770000_01D_MN".to_string()),
            "ESBC00DNK_R_20201770000_01D_MN",
        );

        // explicit prefix is accepted too
        assert_eq!(
            QcIndexing::from_str("custom:base").unwrap(),
            QcIndexing::Custom("base".to_string())
        );

        // ':' inside a custom alias, not a known prefix
        assert_round_trip(QcIndexing::Custom("site:A".to_string()), "site:A");
    }

    #[test]
    fn ambiguous_custom_from_str() {
        // custom aliases that look like another indexing are escaped
        for name in [
            "agency:GRGS",
            "antenna:x",
            "receiver:x",
            "operator:x",
            "custom:x",
        ] {
            assert_round_trip(
                QcIndexing::Custom(name.to_string()),
                &format!("custom:{}", name),
            );
        }
    }

    #[test]
    fn names_are_kept_as_is() {
        // prefixes are case sensitive: this is a custom alias
        assert_round_trip(QcIndexing::Custom("Agency:GRGS".to_string()), "Agency:GRGS");

        // spaces are kept
        assert_round_trip(QcIndexing::Agency(" IGS ".to_string()), "agency: IGS ");

        // nested prefixes belong to the name
        assert_round_trip(
            QcIndexing::Agency("agency:x".to_string()),
            "agency:agency:x",
        );
    }

    #[test]
    fn empty_names_from_str() {
        for text in [
            "",
            "antenna:",
            "receiver:",
            "operator:",
            "agency:",
            "custom:",
        ] {
            assert!(
                matches!(QcIndexing::from_str(text), Err(QcIndexingError::EmptyName)),
                "{}",
                text
            );
        }
    }

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
    fn test_receiver_model_sn_indexing() {
        let test = QcIndexing::from_receiver_model_sn("SEPT POLARX5", "3047937");
        assert_eq!(
            test,
            QcIndexing::Receiver("SEPT POLARX5 #3047937".to_string())
        );

        let test = QcIndexing::from_receiver_model_sn("SEPT POLARX5", "  ");
        assert_eq!(test, QcIndexing::Receiver("SEPT POLARX5".to_string()));
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
