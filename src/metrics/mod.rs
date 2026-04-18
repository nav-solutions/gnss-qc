pub mod record;
pub mod time;

use record::QcRecordMetrics;
use time::QcTemporalMetrics;

/// [QcMetrics] are statistical analysis to present the
/// "quality" of the context, mostly prior precise navigation.
pub struct QcMetrics {
    /// [QcRecordMetrics] are statistical analysis of the data frame
    /// itself, and applies to any data frames.
    pub record: QcRecordMetrics,

    /// [QcTemporalMetrics] are statistical analysis of the
    /// temporal axis and sampling rate of a data frame.
    /// These metrics apply to any frame sampled in time.
    pub temporal: Option<QcTemporalMetrics>,
}
