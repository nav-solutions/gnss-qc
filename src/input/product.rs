//! Input products

use rinex::Rinex;

#[cfg(feature = "sp3")]
use sp3::prelude::SP3;

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

    /// Static [SP3] file
    SP3(SP3),
}

impl InputProduct {
    /// Returns reference to underlying [Rinex] when that applies
    pub fn as_rinex(&self) -> Option<&Rinex> {
        match self {
            Self::RINEX(rinex) => Some(rinex),
            _ => None,
        }
    }

    /// Returns reference to underlying [Rinex] when that applies
    pub fn as_rinex_mut(&mut self) -> Option<&mut Rinex> {
        match self {
            Self::RINEX(rinex) => Some(rinex),
            _ => None,
        }
    }
    
    /// Returns reference to underlying [SP3] when that applies
    #[cfg(feature = "sp3")]
    pub fn as_sp3(&self) -> Option<&SP3> {
        match self {
            Self::SP3(sp3) => Some(sp3),
            _ => None,
        }
    }
    
    /// Returns reference to underlying [SP3] when that applies
    #[cfg(feature = "sp3")]
    pub fn as_sp3_mut(&mut self) -> Option<&mut SP3> {
        match self {
            Self::SP3(sp3) => Some(sp3),
            _ => None,
        }
    }
}
