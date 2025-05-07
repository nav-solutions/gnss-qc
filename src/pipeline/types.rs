use crate::pipeline::errors::PipelineError;

/// All types of data we manipulate within the pipeline
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum QcDataType {
    /// [QcDataTypes::QcWrappedData] streamed by QcContextSerializer
    QcWrappedData,

    /// [QcDataTypes::QcObservationData]
    QcObservationData,

    /// [QcDataTypes::QcEphemerisData]
    QcEphemerisData,
}

impl std::fmt::Display for QcDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QcWrappedData => write!(f, "wrapped"),
            Self::QcEphemerisData => write!(f, "ephemeris"),
            Self::QcObservationData => write!(f, "observations"),
        }
    }
}

impl std::fmt::LowerHex for QcDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QcWrappedData => write!(f, "wrapped"),
            Self::QcEphemerisData => write!(f, "eph"),
            Self::QcObservationData => write!(f, "obs"),
        }
    }
}

impl std::str::FromStr for QcDataType {
    type Err = PipelineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_lowercase();

        match s.as_str() {
            "wrapped" => Ok(Self::QcWrappedData),
            "eph" | "ephemeris" => Ok(Self::QcEphemerisData),
            "obs" | "observation" | "observations" => Ok(Self::QcObservationData),
            _ => Err(PipelineError::InvalidDataType),
        }
    }
}

#[cfg(test)]
mod test {

    use super::QcDataType;
    use std::str::FromStr;

    #[test]
    fn data_types_parsing() {
        for (dtype, expected) in [
            ("wrapped", QcDataType::QcWrappedData),
            ("obs", QcDataType::QcObservationData),
            ("observation", QcDataType::QcObservationData),
            ("observations", QcDataType::QcObservationData),
            ("eph", QcDataType::QcEphemerisData),
            ("ephemeris", QcDataType::QcEphemerisData),
        ] {
            let parsed = QcDataType::from_str(dtype).unwrap();

            assert_eq!(parsed, expected);
        }
    }
}
