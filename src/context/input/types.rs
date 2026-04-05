//! Input Types definition
use rinex::prelude::RinexType;

use crate::{
    context::input::Error,
};

/// Supported input product types
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InputType {
    /// Signal observations as RINEX observation data
    Observations,

    /// Meteo sensors observations as RINEX meteo data
    MeteoObservations,

    /// Broadcast navigation messages as RINEX navigation data
    BroadcastMessages,

    #[cfg(feature = "sp3")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
    /// High precision orbital states as SP3 data
    HighPrecisionOrbits,
}

impl std::fmt::Display for InputType {
    /// Format [InputType] in a readable fashion
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Observations => write!(f, "Observation data"),
            Self::MeteoObservations => write!(f, "Meteo data"),
            Self::BroadcastMessages => write!(f, "Broadcast navigation messages"),
            #[cfg(feature = "sp3")]
            Self::HighPrecisionOrbits => write!(f, "High precision orbital data"),
        }
    }
}

impl std::fmt::LowerHex for InputType {
    /// Format [InputType] in a shortened fashion
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Observations => write!(f, "obs"),
            Self::MeteoObservations => write!(f, "met"),
            Self::BroadcastMessages => write!(f, "brdc"),
            #[cfg(feature = "sp3")]
            Self::HighPrecisionOrbits => write!(f, "sp3"),
       }
    }
}

impl std::str::FromStr for InputType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        let lowered = trimmed.to_lowercase();
        match lowered.as_str() {
            "obs" | "observation" => Ok(Self::Observation),
            "met" | "meteo" => Ok(Self::MeteoObservation),
            "nav" | "brdc" | "navigation" => Ok(Self::BroadcastNavigation),
            #[cfg(feature = "sp3")]
            "sp3" => Ok(Self::HighPrecisionOrbit),
            _ => Err(Error::NonSupportedFormat),
        }
    }
}

impl From<RinexType> for ProductType {
    fn from(rinex: RinexType) -> Result<Self, Error> {
        match rinex {
            RinexType::ObservationData => Ok(Self::Observation),
            RinexType::NavigationData => Ok(Self::BroadcastNavigation),
            RinexType::MeteoData => Ok(Self::MeteoObservation),
            _ => {
                Err(Error::NonSupportedFormat)
            }
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn from_str() {
        for (format, input_type, formatted, hex) in [
            ("obs", InputType::Observations, "Observation data", "obs"),
            ("meteo", InputType::MeteoObservations, "Meteo data", "met"),
            ("nav", InputType::BroadcastMessages, "Broadcast navigation messages", "brdc"),
            ("brdc", InputType::BroadcastMessages, "Broadcast navigation messages", "brdc"),
            ("sp3", InputType::BroadcastMessages, "High precision orbital data", "sp3"),
        ] {
            let input = InputType::from_str(format)
                .unwrap_or_else(|| {
                    panic!("failed to parse valid format \"{}\"", format);
                });

            assert_eq!(input, input_type);
            assert_eq!(format!("{}", input), formatted);
            assert_eq!(format!("{:x}", input), hex);
        }
    }

    #[test]
    fn from_rinex_type() {
        for (rinex_type, input_type) in [
            (RinexType::ObservationData, InputType::Observations),
            (RinexType::NavigationData, InputType::BroadcastMessages),
            (RinexType::MeteoData, InputType::MeteoObservations),
        ] {
            let input = InputType::from(rinex_type)
                .unwrap_or_else(|| {
                    panic!("failed to guess from valid RINEX format {}", rinex_type);
                });

            assert_eq!(input, input_type);
        }
    }

}
