use crossbeam_channel::{Receiver, Sender};

use crate::{pipeline::nodes::Node, prelude::QcProductType, serializer::data::QcSerializedItem};

/// [QcProductTypeFilter] drops anything but a single [QcProductType]
pub struct QcProductTypeFilter {
    name: String,
    product_type: QcProductType,
    rx: Receiver<QcSerializedItem>,
    tx: Sender<QcSerializedItem>,
}

impl QcProductTypeFilter {
    pub fn new(
        name: &str,
        product_type: QcProductType,
        rx: Receiver<QcSerializedItem>,
        tx: Sender<QcSerializedItem>,
    ) -> QcProductTypeFilter {
        Self {
            rx,
            tx,
            product_type,
            name: name.to_string(),
        }
    }
}

impl Node<QcSerializedItem, QcSerializedItem> for QcProductTypeFilter {
    fn name(&self) -> &str {
        &self.name
    }

    fn receiver(&mut self) -> &mut Receiver<QcSerializedItem> {
        &mut self.rx
    }

    fn sender(&mut self) -> &mut Sender<QcSerializedItem> {
        &mut self.tx
    }

    fn task(&mut self, input: QcSerializedItem) -> Option<QcSerializedItem> {
        match input {
            QcSerializedItem::Ephemeris(data) => {
                if data.product_type == self.product_type {
                    Some(QcSerializedItem::Ephemeris(data))
                } else {
                    None
                }
            }
            QcSerializedItem::RINEXHeader(data) => {
                if data.product_type == self.product_type {
                    Some(QcSerializedItem::RINEXHeader(data))
                } else {
                    None
                }
            }
            QcSerializedItem::SP3Header(data) => {
                if self.product_type == QcProductType::PreciseOrbit {
                    Some(QcSerializedItem::SP3Header(data))
                } else {
                    None
                }
            }
            QcSerializedItem::Signal(data) => {
                if data.product_type == self.product_type {
                    Some(QcSerializedItem::Signal(data))
                } else {
                    None
                }
            }
        }
    }
}
