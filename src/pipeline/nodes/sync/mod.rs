use crossbeam_channel::{Receiver, Sender};

use crate::serializer::data::QcSerializedItem;

use crate::pipeline::nodes::Node;

/// [QcEpochStreamSynchronizer] will align all input streams in time out the output side
pub struct QcEpochStreamSynchronizer<const N: usize> {
    name: String,
    rx_ports: [Receiver<QcSerializedItem>; N],
    tx: Sender<QcSerializedItem>,
}

impl<const N: usize> Node<N, QcSerializedItem, QcSerializedItem> for QcEpochStreamSynchronizer<N> {
    fn name(&self) -> &str {
        &self.name
    }

    fn receiver(&mut self, port_index: usize) -> &mut Receiver<QcSerializedItem> {
        &mut self.rx_ports[port_index]
    }

    fn sender(&mut self) -> &mut Sender<QcSerializedItem> {
        &mut self.tx
    }

    fn task(&mut self, input: QcSerializedItem) -> Option<QcSerializedItem> {
        None
    }
}
