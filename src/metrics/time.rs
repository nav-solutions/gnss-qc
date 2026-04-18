use crate::{
    prelude::Duration,
    utils::time::{TemporalArc, TemporalAxis},
};

/// [QcTemporalMetrics] are statistical analysis of the
/// temporal axis and sampling rate of a data frame.
pub struct QcTemporalMetrics {
    /// Number of temporal gaps: discontinuities
    /// in the sampling rate
    pub num_gaps: usize,

    /// Duration of largest gap
    pub largest_gap: Duration,

    /// Mean sampling period, expressed as [Duration]
    pub sampling_period: Duration,

    /// standard deviation of the sampling period, as [Duration]
    pub stddev: f64,

    /// Duty cycle: fraction of the temporal axis with
    /// usable data, and the total temporal axis.
    pub duty_cycle: f64,
}
