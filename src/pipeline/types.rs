use crate::pipeline::errors::PipelineError;

/// All types of data we manipulate within the pipeline
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum QcPipelineTypes {
    /// [QcPipelineTypes::QcWrappedData] streamed by QcContextSerializer
    QcWrappedData,
    
    /// [QcPipelineTypes::QcObservationData]
    QcObservationData,

    /// [QcPipelineTypes::QcEphemerisData]
    QcEphemerisData,
}

impl std::fmt::Display for QcPipelineTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QcWrappedData => write!(f, "wrapped"),
            Self::QcEphemerisData => write!(f, "ephemeris"),
            Self::QcObservationData => write!(f, "observations"),
        }
    }
}

impl std::fmt::LowerHex for QcPipelineTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QcWrappedData => write!(f, "wrapped"),
            Self::QcEphemerisData => write!(f, "eph"),
            Self::QcObservationData => write!(f, "obs"),
        }
    }
}

impl std::str::FromStr for QcPipelineTypes {
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

    use std::str::FromStr;
    use super::QcPipelineTypes;

    #[test]
    fn data_types_parsing() {
        for (dtype, expected) in [
            ("wrapped", QcPipelineTypes::QcWrappedData),
            ("obs", QcPipelineTypes::QcObservationData),
            ("observation", QcPipelineTypes::QcObservationData),
            ("observations", QcPipelineTypes::QcObservationData),
            ("eph", QcPipelineTypes::QcEphemerisData),
            ("ephemeris", QcPipelineTypes::QcEphemerisData),
        ] {
            let parsed = QcPipelineTypes::from_str(dtype)
                .unwrap();

            assert_eq!(parsed, expected);
        }
    }
}