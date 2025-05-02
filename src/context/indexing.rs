use crate::error::QcError;

use serde::{Deserialize, Serialize};

pub(crate) enum QcIndexer {}

/// [QcIndexing] is used to index data and be able to differentiate two identical product types between each other.
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Hash, Serialize, Deserialize)]
pub enum QcIndexing {
    /// No clear identifier found or existing for this type of product
    None,

    /// Identified by the name, model or serial number of the GNSS-Receiver.
    /// This is used by data sources.
    GnssReceiver(String),

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
