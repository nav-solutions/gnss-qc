use crate::prelude::Constellation;
use crate::serializer::data::{QcSerializedEphemeris, QcSerializedSignal};
use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct QcElevationSNRConstellationReport {}

impl QcElevationSNRConstellationReport {
    pub fn add_signal_contribution(&mut self, signal: &QcSerializedSignal) {}
    pub fn add_ephemeris_message(&mut self, msg: &QcSerializedEphemeris) {}
}

#[derive(Clone, Default)]
pub struct QcElevationSNRReport {
    /// SNR/Elevation report per [Constellation]
    pub constell_data: HashMap<Constellation, QcElevationSNRConstellationReport>,
}

impl QcElevationSNRReport {
    pub fn add_signal_contribution(&mut self, signal: &QcSerializedSignal) {}
    pub fn add_ephemeris_message(&mut self, msg: &QcSerializedEphemeris) {}
}
