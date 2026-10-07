//! Input user preferences

pub struct QcInputConfig {
    /// Prefered Indexing method for signal streams.
    /// When set to none, the library will index automatically based on the input stream.
    pub signal_indexing: Option<InputIndexing>,

    /// Prefered Indexing method for ephemeris streams.
    /// When set to none, the library will index automatically based on the input stream.
    pub ephemeris_indexing: Option<InputIndexing>,

    /// Prefered Indexing method for spatial states providers.
    /// When set to none, the library will index automatically based on the input stream.
    pub state_indexing: Option<InputIndexing>,
}
