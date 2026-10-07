//! Input user preferences
use crate::input::indexing::QcIndexing;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct QcInputConfig {
    /// Prefered Indexing method for signal streams.
    /// When set to none, the library will index automatically based on the input stream.
    pub signal_indexing: Option<QcIndexing>,

    /// Prefered Indexing method for ephemeris streams.
    /// When set to none, the library will index automatically based on the input stream.
    pub ephemeris_indexing: Option<QcIndexing>,

    /// Prefered Indexing method for spatial states providers.
    /// When set to none, the library will index automatically based on the input stream.
    pub state_indexing: Option<QcIndexing>,
}
