use crate::prelude::Duration;

/// [QcRecordMetrics] are statistical analysis of any record frame.
pub struct QcRecordMetrics {
    /// Indexing of this data frame.
    pub indexing: String,

    /// Provider of this data frame
    pub provider: String,

    /// Number of items in this data frame
    pub size: usize,
}
