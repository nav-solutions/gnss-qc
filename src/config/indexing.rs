use crate::prelude::QcError;
use serde::{Deserialize, Serialize};

/// Input data prefered indexing method (classification).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum QcPreferedIndexing {
    /// Let the framework figure it out
    #[default]
    Auto,

    /// Indexing by GNSS-Receiver (RX) name or model will be prefered for all signal sources.
    /// If such information is not present in the signal source, we will still use a subsidary option though.
    GnssReceiver,

    /// Indexing by name of operator
    Operator,

    /// Indexing by agency name. If your signal observations are not tied to an agency publisher,
    /// we will still use a subsidary signal indexing method though.
    Agency,
}

impl std::str::FromStr for QcPreferedIndexing {
    type Err = QcError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_ascii_lowercase();

        match s.as_str() {
            "auto" => Ok(Self::Auto),
            "rx" | "gnss" => Ok(Self::GnssReceiver),
            "op" | "operator" => Ok(Self::Operator),
            "ag" | "agency" => Ok(Self::Agency),
            _ => Err(QcError::InvalidIndexingMethod),
        }
    }
}
