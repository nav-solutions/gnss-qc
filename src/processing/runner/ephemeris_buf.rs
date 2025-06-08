use crate::{prelude::Epoch, serializer::data::QcSerializedEphemeris};

pub struct EphemerisBuffer<'a> {
    inner: Vec<QcSerializedEphemeris<'a>>,
}

impl<'a> EphemerisBuffer<'a> {
    pub fn new() -> Self {
        Self {
            inner: Vec::with_capacity(16),
        }
    }

    pub fn update(&mut self, latest: Epoch) {
        self.inner
            .retain(|item| item.data.ephemeris.is_valid(item.data.sv, latest));
    }

    pub fn latch(&mut self, item: QcSerializedEphemeris<'a>) {
        self.inner.push(item);
    }
}
