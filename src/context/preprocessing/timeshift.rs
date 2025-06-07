use crate::prelude::{GnssAbsoluteTime, QcContext, TimeScale, Timeshift};

impl Timeshift for QcContext {
    fn timeshift(&self, solver: &GnssAbsoluteTime, target: TimeScale) -> Self
    where
        Self: Sized,
    {
        self.clone()
    }

    fn timeshift_mut(&mut self, solver: &GnssAbsoluteTime, target: TimeScale) {}
}
