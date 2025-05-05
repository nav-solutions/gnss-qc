use crate::prelude::Epoch;

#[derive(Debug, Clone, Default)]
pub struct TemporalData {
    pub data: Vec<f64>,
    pub epochs: Vec<Epoch>,
}

impl TemporalData {
    pub fn new(t: Epoch, data: f64) -> Self {
        let mut x = Vec::with_capacity(8);
        let mut y = Vec::with_capacity(8);

        x.push(t);
        y.push(data);

        Self { epochs: x, data: y }
    }

    pub fn push(&mut self, t: Epoch, data: f64) {
        self.epochs.push(t);
        self.data.push(data);
    }

    pub fn epochs(&self) -> &[Epoch] {
        &self.epochs
    }

    pub fn data(&self) -> &[f64] {
        &self.data
    }
}
