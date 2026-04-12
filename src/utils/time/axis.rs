use crate::{
    prelude::{Epoch, TimeSeries},
    utils::TemporalArc,
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
            args: Vec::with_capacity(size)
        }
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
    pub fn largest_gap(&self) -> Duration {

    }

    /// Adds the following [Epoch] to this [TemporalAxis].
    pub fn add_epoch(&mut self, epoch: Epoch) {
        // determine whether this [Epoch] belongs in an existing arc or not
        let mut is_wrapped = false;

        for arc in self.arcs.iter() {
            
        }

        if is_wrapped {

        }
    }
}


pub struct TemporalAxisIter<'a> {

}

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
}
