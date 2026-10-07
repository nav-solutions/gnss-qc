#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use rinex::prelude::RinexType;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum QcRINEXType {
    /// Signal observations
    Observation,
    /// Meteo sensor observations
    Meteo,
    /// Navigation messages
    Navigation,
    /// (Ground or Spaceborn) clock states
    Clock,
}

impl QcRINEXType {
    fn from_rinex_type(rtype: &RinexType) -> Self {
        match rtype {
            RinexType::ObservationData => Self::Observation,
            RinexType::MeteoData => Self::Meteo,
            RinexType::NavigationData => Self::Navigation,
            RinexType::ClockData => Self::Clock,
            _ => panic!("replaced by dedicated lib"),
        }
    }
}

/// Types of static products supported by this library
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum QcProductType {
    /// RINEX file, from our RINEX parser <https://github.com/nav-solutions/rinex>
    RINEX(QcRINEXType),

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
