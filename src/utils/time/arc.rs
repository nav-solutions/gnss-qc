use crate::prelude::{Epoch, TimeSeries};

/// [TemporalArc] represents a continuous (no gap)
/// space of evenly spaced [Epoch]s.
/// A [TemporalArc] can be iterated both forward and backwards.
pub struct TemporalArc {
    pub start: Epoch,
    pub step: Duration,
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

    /// Creates an [TimeSeries] we can then iterate, either
    /// forward or backwards.
    pub fn into_iter(&self) -> TimeSeries {
        TimeSeries::inclusive(self.start, self.end(), self.duration);
    }

    pub fn contains(&self, epoch: &Epoch) -> bool {
        self.start >= epoch && self.end() <= epoch
    }
}
