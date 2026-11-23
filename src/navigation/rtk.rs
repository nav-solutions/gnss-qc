//! RTK reference station
use gnss_rtk::prelude::{Candidate, Epoch, RTKBase};

use crate::io::input::indexing::QcIndexing;

/// [RTKBase] implementation: the base station is one of the signal sources,
/// its candidates are refreshed before each resolution (same epoch as the rover).
pub struct QcRtkBase {
    pub indexing: QcIndexing,
    pub position_ecef_m: (f64, f64, f64),
    pub candidates: Vec<Candidate>,
}

impl RTKBase for QcRtkBase {
    fn name(&self) -> String {
        self.indexing.to_string()
    }

    fn new_epoch(&mut self, _: Epoch) {}

    fn observe(&self, _: Epoch) -> Vec<Candidate> {
        self.candidates.clone()
    }

    fn reference_position_ecef_m(&self, _: Epoch) -> (f64, f64, f64) {
        self.position_ecef_m
    }
}
