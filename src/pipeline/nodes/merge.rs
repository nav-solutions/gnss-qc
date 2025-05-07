use crossbeam_channel::{Receiver, Sender};

use crate::{
    pipeline::nodes::Node,
    serializer::data::{QcSerializedEphemeris, QcSerializedItem},
};

/// [QcStreamMerge] merges two streams into a single one.
pub struct QcStreamMerge {
    name: String,
    rx: [Receiver<QcSerializedItem>; 2],
    tx: Sender<QcSerializedItem>,
}

impl QcStreamMerge {
    pub fn new(
        name: &str,
        rx: [Receiver<QcSerializedItem>; 2],
        tx: Sender<QcSerializedItem>,
    ) -> QcStreamMerge {
        Self {
            rx,
            tx,
            name: name.to_string(),
        }
    }
}

impl Node<2, QcSerializedItem, QcSerializedItem> for QcStreamMerge {
    fn name(&self) -> &str {
        &self.name
    }

    fn receiver(&mut self, index: usize) -> &mut Receiver<QcSerializedItem> {
        &mut self.rx[index]
    }

    fn sender(&mut self) -> &mut Sender<QcSerializedItem> {
        &mut self.tx
    }

    fn task(&mut self, input: QcSerializedItem) -> Option<QcSerializedItem> {
        match input {
            _ => None,
        }
    }
}
