use crossbeam_channel::{Receiver, Sender};

use crate::{pipeline::nodes::Node, prelude::QcProductType, serializer::data::QcSerializedItem};

/// [QcFilenameStreamFilter] drops anything but a single [QcProductType]
pub struct QcFilenameStreamFilter {
    name: String,
    filename: String,
    rx: Receiver<QcSerializedItem>,
    tx: Sender<QcSerializedItem>,
}

impl QcFilenameStreamFilter {
    pub fn new(
        name: &str,
        filename: String,
        rx: Receiver<QcSerializedItem>,
        tx: Sender<QcSerializedItem>,
    ) -> Self {
        Self {
            rx,
            tx,
            filename,
            name: name.to_string(),
        }
    }
}

impl Node<1, QcSerializedItem, QcSerializedItem> for QcFilenameStreamFilter {
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
                if data.filename == self.filename {
                    Some(QcSerializedItem::Ephemeris(data))
                } else {
                    None
                }
            }
            QcSerializedItem::RINEXHeader(data) => {
                if data.filename == self.filename {
                    Some(QcSerializedItem::RINEXHeader(data))
                } else {
                    None
                }
            }
            QcSerializedItem::SP3Header(data) => {
                if data.filename == self.filename {
                    Some(QcSerializedItem::SP3Header(data))
                } else {
                    None
                }
            }
            QcSerializedItem::Signal(data) => {
                if data.filename == self.filename {
                    Some(QcSerializedItem::Signal(data))
                } else {
                    None
                }
            }
        }
    }
}
