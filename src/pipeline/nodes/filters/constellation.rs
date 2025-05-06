use crossbeam_channel::{Receiver, Sender};

use crate::{pipeline::nodes::Node, prelude::Constellation, serializer::data::QcSerializedItem};

/// [QConstellationStreamFilter] drops anything but a single constellation
pub struct QConstellationStreamFilter {
    name: String,
    constellation: Constellation,
    rx: Receiver<QcSerializedItem>,
    tx: Sender<QcSerializedItem>,
}

impl QConstellationStreamFilter {
    pub fn new(
        name: &str,
        constellation: Constellation,
        rx: Receiver<QcSerializedItem>,
        tx: Sender<QcSerializedItem>,
    ) -> QConstellationStreamFilter {
        Self {
            rx,
            tx,
            constellation,
            name: name.to_string(),
        }
    }
}

impl Node<QcSerializedItem, QcSerializedItem> for QConstellationStreamFilter {
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
            QcSerializedItem::Ephemeris(eph) => {
                if eph.data.sv.constellation == self.constellation {
                    Some(QcSerializedItem::Ephemeris(eph))
                } else {
                    None
                }
            }
            QcSerializedItem::Signal(signal) => {
                if signal.data.sv.constellation == self.constellation {
                    Some(QcSerializedItem::Signal(signal))
                } else {
                    None
                }
            }
            QcSerializedItem::RINEXHeader(header) => {
                if let Some(constellation) = header.data.constellation {
                    if constellation == Constellation::Mixed || constellation == self.constellation
                    {
                        Some(QcSerializedItem::RINEXHeader(header))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            QcSerializedItem::SP3Header(header) => {
                if header.data.constellation == Constellation::Mixed
                    || header.data.constellation == self.constellation
                {
                    Some(QcSerializedItem::SP3Header(header))
                } else {
                    None
                }
            }
        }
    }
}
