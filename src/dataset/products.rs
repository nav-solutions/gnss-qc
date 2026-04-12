//! Input products
use rinex::prelude::RinexType;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum QcInputProduct {
    /// All supported RINEX types
    RINEX(RinexType),

    /// High precision Orbit files.
    /// Can serve to overwrite the orbital states
    /// described by unprecise radio messages.
    #[cfg(feature = "sp3")]
    SP3,
}

