use crate::serializer::data::QcSerializedPreciseState;

pub struct PreciseStateBuffer {
    inner: Vec<QcSerializedPreciseState>,
}

impl PreciseStateBuffer {
    pub fn new() -> Self {
        Self {
            inner: Vec::with_capacity(16),
        }
    }

    pub fn latch(&mut self, item: QcSerializedPreciseState) {
        self.inner.push(item);
    }
}
