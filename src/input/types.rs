#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Types of static products supported by this library
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum InputProductType {
    /// RINEX file, from our RINEX parser <https://github.com/nav-solutions/rinex>
    RINEX,

    /// SP3 file, from our SP3 parser <https://github.com/nav-solutions/sp3>
    #[cfg(feature = "sp3")]
    SP3,

    /// TEC map encoded as special RINEX format, from our IONEX parser
    /// <https://github.com/nav-solutions/ionex>
    #[cfg(feature = "ionex")]
    IONEX,

    /// Static antenna calibration parameters for higher precision, encoded as
    /// special RINEX format. Supported by our ANTEX parser
    /// <https://github.com/nav-solutions/antex>
    #[cfg(feature = "antex")]
    ANTEX,
}
