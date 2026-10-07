#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum InputProductType {
    /// RINEX file, from our RINEX parser <https://github.com/nav-solutions/rinex>
    RINEX,

    /// SP3 file, from our SP3 parser <https://github.com/nav-solutions/sp3>
    #[cfg(feature = "sp3")]
    SP3,
}
