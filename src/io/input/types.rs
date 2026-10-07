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
    /// Antenna calibration database (ANTEX)
    Antenna,
}

impl QcRINEXType {
    pub fn from_rinex_type(rtype: &RinexType) -> Self {
        match rtype {
            RinexType::ObservationData => Self::Observation,
            RinexType::MeteoData => Self::Meteo,
            RinexType::NavigationData => Self::Navigation,
            RinexType::ClockData => Self::Clock,
            RinexType::AntennaData => Self::Antenna,
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
}
