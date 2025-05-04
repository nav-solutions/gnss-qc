use maud::{html, Markup, Render};
use rinex::prelude::{Duration, Epoch, Rinex};

#[cfg(feature = "sp3")]
use sp3::SP3;

/// [SamplingGap]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct SamplingGap {
    /// Last sampling [Epoch] latched
    pub start: Epoch,
    /// Dead time [Duration]
    pub duation: Duration,
}

/// [SamplingGap] analysis, present on perturbated sampling
pub struct SamplingGapAnalysis {
    /// [SamplingGap]s that were identified
    pub gaps: Vec<SamplingGap>,

    /// Shortest [SamplingGap] duration
    pub shortest: Duration,

    /// Longest [SamplingGap] duration
    pub longest: Duration,
}

/// [Sampling] analysis may apply to any time domain datasets
pub struct Sampling {
    /// Total number of [Epoch]s processed
    pub total_epochs: usize,

    /// First [Epoch] ever processed
    pub first_epoch: Epoch,

    /// Last [Epoch] ever processed
    pub last_epoch: Epoch,

    /// Timeframe [Duration]
    pub duration: Duration,

    /// Announced sampling period (if any)
    pub expected_sampling_interval: Option<Duration>,

    /// Dominant sampling rate that was identified
    pub dominant_sample_rate: Option<f64>,

    /// [SamplingGapAnalysis] (if any)
    pub gap_analysis: Option<SamplingGapAnalysis>,
}
