//! Error types definition
use thiserror::Error;

use qc_traits::MergeError;

use rinex::error::ParsingError as RinexParsingError;

#[cfg(feature = "sp3")]
use sp3::Error as SP3Error;

/// Context Error
#[derive(Debug, Error)]
pub enum QcError {
    #[error("non supported file format")]
    NonSupportedFileFormat,

    #[error("invalid indexing method")]
    InvalidIndexingMethod,

    #[error("failed to determine filename")]
    FileNameDetermination,

    #[error("failed to extend context")]
    Merge(#[from] MergeError),

    #[error("unknown / non supported product type")]
    UnknownProductType,

    #[error("invalid nav filter")]
    InvalidNavFilter,

    #[error("RINEX parsing error: {0}")]
    RinexParsing(#[from] RinexParsingError),

    #[cfg(feature = "sp3")]
    #[error("SP3 parsing error: {0}")]
    SP3Parsing(#[from] SP3Error),

    #[error("non supported signal")]
    NonSupportedSignal,
}
