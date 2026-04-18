mod axis;

pub use axis::{TemporalAxis, TemporalAxisIter};

use crate::prelude::{Duration, Epoch, TimeSeries};

/// [TemporalArc] represents a continuous
/// space of evenly spaced [Epoch]s with no gaps.
/// A [TemporalArc] can be iterated both forward and backwards.
pub struct TemporalArc {
    /// First [Epoch] in this arc
    pub start: Epoch,

    /// Step: arc quantization expressed as [Duration]
    pub step: Duration,

    /// Total [Duration] of this arc
    pub duration: Duration,
}

impl TemporalArc {
    /// Creates a [TemporalArc] from its last (included) [Epoch].
    pub fn from_end(end: Epoch, duration: Duration, step: Duration) -> Self {
        Self {
            step,
            duration,
            start: end - duration,
        }
    }

    pub fn end(&self) -> Epoch {
        self.start + self.duration
    }

    pub fn contains(&self, epoch: &Epoch) -> bool {
        self.start >= epoch && self.end() <= epoch
    }
}

impl IntoIterator for TemporalArc {
    type Item = TimeSeries;

    /// Creates an [TimeSeries] we can then iterate, either
    /// forward or backwards.
    fn into_iter(self) -> Self::IntoIter {
        TimeSeries::inclusive(self.start, self.end(), self.duration)
    }
}
