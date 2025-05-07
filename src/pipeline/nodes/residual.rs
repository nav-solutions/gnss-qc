use crossbeam_channel::{Receiver, Sender};

use crate::{
    pipeline::nodes::Node,
    serializer::data::{QcSerializedEphemeris, QcSerializedItem},
};

/// [QcResidualStreamer] streams (A) - (B)
pub struct QcResidualStreamer {
    name: String,
    rx_a: Receiver<QcSerializedItem>,
    rx_b: Receiver<QcSerializedItem>,
    tx: Sender<QcSerializedEphemeris>,
}

impl QcResidualStreamer {
    pub fn new(
        name: &str,
        rx_a: Receiver<QcSerializedItem>,
        rx_b: Receiver<QcSerializedItem>,
        tx: Sender<QcSerializedEphemeris>,
    ) -> QcResidualStreamer {
        Self {
            rx_a,
            rx_b,
            tx,
            name: name.to_string(),
        }
    }
}

impl Node<2, QcSerializedItem, QcSerializedEphemeris> for QcResidualStreamer {
    fn name(&self) -> &str {
        &self.name
    }

    fn receiver(&mut self, _: usize) -> &mut Receiver<QcSerializedItem> {
        &mut self.rx_a
    }

    fn sender(&mut self) -> &mut Sender<QcSerializedEphemeris> {
        &mut self.tx
    }

    fn task(&mut self, input: QcSerializedItem) -> Option<QcSerializedEphemeris> {
        match input {
            QcSerializedItem::Ephemeris(eph) => Some(eph),
            _ => None,
        }
    }
}
