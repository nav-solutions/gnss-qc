use crossbeam_channel::{Receiver, Sender};

use crate::{pipeline::nodes::Node, prelude::QcIndexing, serializer::data::QcSerializedItem};

/// [QcIndexingStreamFilter] drops anything but an exact [QcIndexing]
pub struct QcIndexingStreamFilter {
    name: String,
    indexing: QcIndexing,
    rx: Receiver<QcSerializedItem>,
    tx: Sender<QcSerializedItem>,
}

impl QcIndexingStreamFilter {
    pub fn new(
        name: &str,
        indexing: QcIndexing,
        rx: Receiver<QcSerializedItem>,
        tx: Sender<QcSerializedItem>,
    ) -> QcIndexingStreamFilter {
        Self {
            rx,
            tx,
            indexing,
            name: name.to_string(),
        }
    }
}

impl Node<1, QcSerializedItem, QcSerializedItem> for QcIndexingStreamFilter {
    fn name(&self) -> &str {
        &self.name
    }

    fn receiver(&mut self, _: usize) -> &mut Receiver<QcSerializedItem> {
        &mut self.rx
    }

    fn sender(&mut self) -> &mut Sender<QcSerializedItem> {
        &mut self.tx
    }

    fn task(&mut self, input: QcSerializedItem) -> Option<QcSerializedItem> {
        match input {
            QcSerializedItem::Ephemeris(data) => {
                if data.indexing == self.indexing {
                    Some(QcSerializedItem::Ephemeris(data))
                } else {
                    None
                }
            }
            QcSerializedItem::RINEXHeader(data) => {
                if data.indexing == self.indexing {
                    Some(QcSerializedItem::RINEXHeader(data))
                } else {
                    None
                }
            }
            QcSerializedItem::SP3Header(data) => {
                if data.indexing == self.indexing {
                    Some(QcSerializedItem::SP3Header(data))
                } else {
                    None
                }
            }
            QcSerializedItem::Signal(data) => {
                if data.indexing == self.indexing {
                    Some(QcSerializedItem::Signal(data))
                } else {
                    None
                }
            }
        }
    }
}
