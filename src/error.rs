//! Error types definition
use thiserror::Error;

use qc_traits::MergeError;

use anise::{
    almanac::{metaload::MetaAlmanac, metaload::MetaAlmanacError, planetary::PlanetaryDataError},
    errors::AlmanacError,
};

/// Context Error
#[derive(Debug, Error)]
pub enum Error {
    #[error("almanac error: {0}")]
    Almanac(#[from] AlmanacError),
    #[error("meta error: {0}")]
    MetaAlmanac(#[from] MetaAlmanacError),
    #[error("planetary data error")]
    PlanetaryData(#[from] PlanetaryDataError),
    #[error("non supported file format")]
    NonSupportedFileFormat,
    #[error("failed to determine filename")]
    FileNameDetermination,
    #[error("failed to extend context")]
    Merge(#[from] MergeError),
    #[error("unknown / non supported product type")]
    UnknownProductType,
}
