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
        self.inner
            .retain(|item| item.data.ephemeris.is_valid(item.data.sv, latest));
    }

    pub fn latch(&mut self, item: &QcSerializedEphemeris) {
        self.inner.push(item.clone());
    }
}
