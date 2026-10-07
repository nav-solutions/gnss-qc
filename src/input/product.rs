//! Input products

use rinex::Rinex;

pub mod key;
pub mod types;
pub mod cfg;

use key::InputProductKey;

/// [InputProduct] definition which is the starting point
/// when arriving from static files (typical use case),
/// and will provide data points that can be fed to the processing pipeline.
pub enum InputProduct {
    /// Static [Rinex] file
    RINEX(Rinex),
}

impl InputProduct {
    /// Returns reference to underlying [Rinex] when that applies
    pub fn as_rinex(&self) -> Option<&Rinex> {
        match self {
            Self::RINEX(rinex) => Some(rinex),
            _ => None,
        }
    }
}
