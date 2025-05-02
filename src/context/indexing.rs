use crate::error::QcError;

use serde::{Deserialize, Serialize};

/// [QcIndexing] is used to index data and be able to differentiate two identical product types between each other.
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Hash, Serialize, Deserialize)]
pub enum QcIndexing {
    /// No clear identifier found or existing for this type of product
    None,

    /// Identified by the name, model or serial number of the GNSS-Receiver.
    /// This is used by data sources.
    GnssReceiver(String),

    /// Identified by the name, model or serial number of the Antenna plugged to a receiver.
    /// This is used in custom setups where multiple identical GNSS receivers us different antennas.
    RxAntenna(String),

    /// Identified by name of the data provider. This is applicable
    /// to all precise products and data produced by GNSS agencies.
    Agency(String),

    /// Identified by name of the operator (person who made the measurement).
    /// This is also "Observer" in RINEX terminology.
    Operator(String),

    /// Identified by the name or calibration ID of a geodetic marker.
    /// This usually applies to laboratories and profesionnal data providers.
    GeodeticMarker(String),

    /// Identified by a custom name that the user manually specified.
    /// This library will not use that option by itself.
    Custom(String),
}

impl QcIndexing {
    /// Unwraps self as [QcIndexing::GnssReceiver] model name, if applicable
    pub fn as_gnss_receiver(&self) -> Option<String> {
        match self {
            Self::GnssReceiver(gnss_rx) => Some(gnss_rx.clone()),
            _ => None,
        }
    }
    /// Unwraps self as [QcIndexing::RxAntenna] model, if applicable
    pub fn as_antenna(&self) -> Option<String> {
        match self {
            Self::RxAntenna(antenna) => Some(antenna.clone()),
            _ => None,
        }
    }

    /// Unwraps self as [QcIndexing::GeodeticMarker] ID, if applicable
    pub fn as_geodetic_marker(&self) -> Option<String> {
        match self {
            Self::GeodeticMarker(marker) => Some(marker.clone()),
            _ => None,
        }
    }

    /// Unwraps self as [QcIndexing::Agency] name, if applicable
    pub fn as_agency(&self) -> Option<String> {
        match self {
            Self::Agency(agency) => Some(agency.clone()),
            _ => None,
        }
    }

    /// Unwraps self as [QcIndexing::Operator] name, if applicable
    pub fn as_operator(&self) -> Option<String> {
        match self {
            Self::Operator(operator) => Some(operator.clone()),
            _ => None,
        }
    }

    /// Unwraps self as [QcIndexing::Custom] name, if applicable
    pub fn as_custom_name(&self) -> Option<String> {
        match self {
            Self::Custom(custom) => Some(custom.clone()),
            _ => None,
        }
    }
}

impl std::fmt::Display for QcIndexing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Agency(agency) => {
                write!(f, "agency name: \"{}\"", agency)
            }
            Self::Operator(operator) => {
                write!(f, "operator name: \"{}\"", operator)
            }
            Self::GeodeticMarker(marker) => {
                write!(f, "geodetic marker: \"{}\"", marker)
            }
            Self::GnssReceiver(model) => {
                write!(f, "receiver model: \"{}\"", model)
            }
            Self::RxAntenna(antenna) => {
                write!(f, "rx-antenna: \"{}\"", antenna)
            }
            Self::Custom(custom) => {
                write!(f, "custom ID: \"{}\"", custom)
            }
            Self::None => {
                write!(f, "Not indexed")
            }
        }
    }
}

impl std::str::FromStr for QcIndexing {
    type Err = QcError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if s.starts_with("agency:") {
            let content = s.split_at(7).1.trim().to_string();
            Ok(QcIndexing::Agency(content))
        } else if s.starts_with("geo:") {
            let content = s.split_at(4).1.trim().to_string();
            Ok(QcIndexing::GeodeticMarker(content))
        } else if s.starts_with("ant:") {
            let content = s.split_at(4).1.trim().to_string();
            Ok(QcIndexing::RxAntenna(content))
        } else if s.starts_with("gnss:") {
            let content = s.split_at(6).1.trim().to_string();
            Ok(QcIndexing::GnssReceiver(content))
        } else if s.starts_with("operator:") {
            let content = s.split_at(9).1.trim().to_string();
            Ok(QcIndexing::Operator(content))
        } else {
            // assume custom
            Ok(QcIndexing::Custom(s.to_string()))
        }
    }
}

#[cfg(test)]
mod test {
    use super::QcIndexing;
    use std::str::FromStr;

    #[test]
    fn qc_identifier_parsing() {
        for (value, expected) in [
            (
                "geo:GEOMARKER",
                QcIndexing::GeodeticMarker("GEOMARKER".to_string()),
            ),
            (
                "gnss:UBLOX-M8T",
                QcIndexing::GnssReceiver("UBLOX-M8T".to_string()),
            ),
            (
                "agency:SERIOUS-AGENCY",
                QcIndexing::Agency("SERIOUS-AGENCY".to_string()),
            ),
            ("operator:MySelf", QcIndexing::Agency("MySelf".to_string())),
        ] {
            let id = QcIndexing::from_str(value).unwrap();

            assert_eq!(id, expected);
        }
    }
}
