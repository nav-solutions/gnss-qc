use crate::{
    metrics::QcTemporalMetrics,
    prelude::{Duration, Epoch, TimeSeries},
    utils::time::TemporalArc,
};

/// [TemporalAxis] describes the total temporal axis
/// for a context and used during an analysis.
pub struct TemporalAxis {
    arcs: Vec<TemporalArc>,
}

impl TemporalAxis {
    /// Pre-allocates an empty [TemporalAxis].
    pub fn with_capacity(size: usize) -> Self {
        Self {
            args: Vec::with_capacity(size),
        }
    }

    /// Creates a new [TemporalAxis] that contains a single [TemporalArc].
    pub fn from_temporal_arc(arc: TemporalArc) -> Self {
        Self { arcs: vec![arc] }
    }

    /// Returns true if this [TemporalAxis] does not event contain
    /// a point in time.
    pub fn is_empty(&self) -> bool {
        self.size() == 0
    }

    /// Returns the total number of continuous, evenly spaced arcs
    /// contained in this [TempoalAxis]
    pub fn size(&self) -> usize {
        self.arcs.len()
    }

    /// Returns the total number of gaps (Arc discontinuity) in this [TemporalAxis]
    pub fn num_gaps(&self) -> usize {
        self.size() - 1
    }

    /// Returns largest gap [Duration] contained in this [TemporalAxis].
    pub fn largest_gap(&self) -> Duration {}

    /// Adds the following [Epoch] to this [TemporalAxis].
    pub fn add_epoch(&mut self, epoch: Epoch) {
        // determine whether this [Epoch] belongs in an existing arc or not
        let mut is_wrapped = false;

        for arc in self.arcs.iter() {}

        if is_wrapped {}
    }

    /// Computes [QcTemporalMetrics] from this [TemporalAxis]
    pub fn compute_metrics(&self) -> QcTemporalMetrics {
        QcTemporalMetrics {
            num_gaps,
            largest_gap,
            sampling_period: sampling_periods.mean()
            stddev: sampling_periods.stddev(),
            duty_cycle:
        }
    }
}

pub struct TemporalAxisIter<'a> {
    ptr: usize,
    arcs: &'a [TemporalArc],
}
