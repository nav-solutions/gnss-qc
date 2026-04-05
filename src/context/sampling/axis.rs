//! Sampling characteristics and definitions
use hifitime::{TimeSeries, Epoch};

/// [TemporalAxis] is associated to any sampled dataset,
/// for example GNSS observations, and is used to define the
/// temporal arc of an analysis.
pub struct TemporalAxis {
    /// Temporal axis is defined as a serie of [TimeSeries]
    /// to support a non-stable sampling rate.
    series: Vec<TimeSeries>,
}

impl TemporalAxis {
    /// Creates a single point [TemporalAxis], used to obtain
    /// a single temporal solution.
    pub fn single_point(epoch: Epoch) -> Self {
        Self {
            series: vec![TimeSeries::inclusive(epoch, epoch, Duration::ZERO)],
        }
    }
    
    /// True if no temporal arc is defined.
    fn is_empty(&self) -> bool {
        self.series.is_empty()
    }
    
    /// Returns total number of temporal arcs.
    fn size(&self) -> usize {
        self.series.len()
    }

    /// Returns first temporal arc of this [TemporalAxis].
    /// A temporal arc is a time window where the sampling rate is stable.
    pub fn first_arc(&self) -> Option<TimeSeries> {
        if self.is_empty() {
            None
        } else {
            Some(self.series[0])
        }
    }
    
    /// Returns last temporal arc of this [TemporalAxis].
    /// A temporal arc is a time window where the sampling rate is stable.
    pub fn last_arc(&self) -> Option<TimeSeries> {
        if self.is_empty() {
            None
        } else {
            Some(self.series[self.size() -1])
        }
    }

    /// Returns total [Duration] of this [TemporalAxis]
    pub fn duration(&self) -> Duration {
        if let Some(first_arc) = self.first_arc() {
            if let Some(last_arc) = self.last_arc() {
                Some(last_arc.last - first_arc.first)
            } else {
                None
            }
        } else {
            None
        }
    }
}   

impl Iterator for TemporalAxis {
    /// 
    fn next(&mut self) -> Option<Epoch> {

    }
}
