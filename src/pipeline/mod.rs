use hifitime::prelude::Epoch;
use crate::input::TimestampedStream;

pub enum QcAlgorithms {
    /// [QcAlgorithms::PrecisePointPositioning] is post-processed 
    /// navigation without ground reference and high precision products.
    /// [QcAlgorithms::PrecisePointPositioning] applies to a single rover.
    PrecisePointPositioning,

    /// [QcAlgorithm::RealTimePositioning] is real-time navigation
    /// using input data streams, without ground references.
    /// We still allow high precision products, mostly static (not temporal products)
    /// to increase the accuracy of the solution.
    /// [QcAlgorithm::RealTimePositioning] applies to a single rover.
    /// You can deploy one processor per rover.
    RealTimePositioning,
    
    /// [QcAlgorithms::RealTimeKinematics] is post-processed or real-time navigation
    /// using at least one rover and at least one static ground reference.
    /// This will solve (_only_) the spatial state the preselected rover.
    /// If the baseline (distance from rover to nearest ground reference) goes above threshold,
    /// the solutions are rejected.
    /// Applies to the preselected rover.
    RealTimeKinematics,
}

pub struct QcProcessor<TS: TimestampedStream> {
    /// Abstract input [TimestampedStream]s where each
    /// item is sampled.
    temporal_streams: Vec<TS>,

    /// Current epoch being processed
    current_epoch: Option<Epoch>,
}

impl <TS: TimestampedStream> QcProcessor<TS> {

    /// Creates a new [QcProcessor]
    pub fn new(temporal_streams: Vec<TS>) {
        Self {
            temporal_streams,
        }
    }
}
