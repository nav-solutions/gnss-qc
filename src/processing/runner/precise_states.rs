use crate::serializer::data::QcSerializedPreciseState;

pub struct PreciseStateBuffer<'a> {
    inner: Vec<QcSerializedPreciseState<'a>>,
}

impl<'a> PreciseStateBuffer<'a> {
    pub fn new() -> Self {
        Self {
            inner: Vec::with_capacity(16),
        }
    }

    pub fn latch(&mut self, item: QcSerializedPreciseState<'a>) {
        self.inner.push(item);
    }
}
