use crossbeam_channel::{Receiver, Sender};

use crate::{
    pipeline::nodes::Node,
    serializer::data::{QcSerializedEphemeris, QcSerializedItem},
};

/// [QcEphemerisStreamer] selects Ephemeris frames within the stream (only)
pub struct QcEphemerisStreamer {
    name: String,
    rx: Receiver<QcSerializedItem>,
    tx: Sender<QcSerializedEphemeris>,
}

impl QcEphemerisStreamer {
    pub fn new(
        name: &str,
        rx: Receiver<QcSerializedItem>,
        tx: Sender<QcSerializedEphemeris>,
    ) -> QcEphemerisStreamer {
        Self {
            rx,
            tx,
            name: name.to_string(),
        }
    }
}

impl Node<QcSerializedItem, QcSerializedEphemeris> for QcEphemerisStreamer {
    fn name(&self) -> &str {
        &self.name
    }

    fn receiver(&mut self) -> &mut Receiver<QcSerializedItem> {
        &mut self.rx
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
