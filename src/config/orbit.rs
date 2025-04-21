use serde::{Deserialize, Serialize};

use crate::config::Error;

/// [QcOrbitPreference] is used to parametrize the navigation process.
#[derive(Default, Copy, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QcOrbitPreference {
    /// Prefer Keplerian navigation from buffered Radio signals.
    /// This option is supported by all navigation compatible setups.
    #[default]
    RadioBroadcast,
    /// Prefer direct or indirect exploitation of precise products.
    /// This only truly applies if "sp3" library feature is activated.
    PreciseProducts,
}

impl std::str::FromStr for QcOrbitPreference {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "sp3" | "precise" => Ok(Self::PreciseProducts),
            "nav" | "rinex" | "brdc" => Ok(Self::RadioBroadcast),
            _ => Err(Error::InvalidOrbitPreference),
        }
    }
}

impl std::fmt::Display for QcOrbitPreference {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::RadioBroadcast => f.write_str("RINEX"),
            Self::PreciseProducts => f.write_str("SP3"),
        }
    }
}
