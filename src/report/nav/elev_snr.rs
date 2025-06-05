use crate::serializer::data::QcSerializedSignal;

#[derive(Clone, Default)]
pub struct QcElevationSNRReport {}

impl QcElevationSNRReport {
    pub fn add_signal_contribution(&mut self, signal: &QcSerializedSignal) {}
}
