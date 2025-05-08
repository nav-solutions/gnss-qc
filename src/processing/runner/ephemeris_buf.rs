use crate::{prelude::Epoch, serializer::data::QcSerializedEphemeris};

pub struct EphemerisBuffer {
    inner: Vec<QcSerializedEphemeris>,
}

impl EphemerisBuffer {
    pub fn new() -> Self {
        Self {
            inner: Vec::with_capacity(16),
        }
    }

    pub fn update(&mut self, latest: Epoch) {
        self.inner.retain(|item| {
            let (sv, toe) = (item.data.sv, item.data.toe);
            item.data.ephemeris.is_valid(sv, latest, toe)
        });
    }

    pub fn latch(&mut self, item: QcSerializedEphemeris) {
        self.inner.push(item);
    }
}
