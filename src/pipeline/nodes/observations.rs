use crossbeam_channel::{Receiver, Sender};

use crate::{
    pipeline::nodes::Node,
    serializer::data::{QcSerializedItem, QcSerializedSignal},
};

/// [QcObservationsStreamer] selects Signal observations within the stream (only)
pub struct QcObservationsStreamer {
    name: String,
    rx: Receiver<QcSerializedItem>,
    tx: Sender<QcSerializedSignal>,
}

impl QcObservationsStreamer {
    pub fn new(
        name: &str,
        rx: Receiver<QcSerializedItem>,
        tx: Sender<QcSerializedSignal>,
    ) -> QcObservationsStreamer {
        Self {
            rx,
            tx,
            name: name.to_string(),
        }
    }
}

impl Node<QcSerializedItem, QcSerializedSignal> for QcObservationsStreamer {
    fn name(&self) -> &str {
        &self.name
    }

    fn receiver(&mut self) -> &mut Receiver<QcSerializedItem> {
        &mut self.rx
    }

    fn sender(&mut self) -> &mut Sender<QcSerializedSignal> {
        &mut self.tx
    }

    fn task(&mut self, input: QcSerializedItem) -> Option<QcSerializedSignal> {
        match input {
            QcSerializedItem::Signal(signal) => Some(signal),
            _ => None,
        }
    }
}
