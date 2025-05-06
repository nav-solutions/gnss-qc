use crossbeam_channel::{Receiver, Sender};

use crate::{pipeline::nodes::Node, prelude::SV, serializer::data::QcSerializedItem};

/// [QcSingleSVStreamer] drops anything but a single [SV]
pub struct QcSingleSVStreamer {
    name: String,
    sv: SV,
    rx: Receiver<QcSerializedItem>,
    tx: Sender<QcSerializedItem>,
}

impl QcSingleSVStreamer {
    pub fn new(
        name: &str,
        sv: SV,
        rx: Receiver<QcSerializedItem>,
        tx: Sender<QcSerializedItem>,
    ) -> QcSingleSVStreamer {
        Self {
            rx,
            tx,
            sv,
            name: name.to_string(),
        }
    }
}

impl Node<QcSerializedItem, QcSerializedItem> for QcSingleSVStreamer {
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
                if eph.data.sv == self.sv {
                    Some(QcSerializedItem::Ephemeris(eph))
                } else {
                    None
                }
            }
            QcSerializedItem::Signal(signal) => {
                if signal.data.sv == self.sv {
                    Some(QcSerializedItem::Signal(signal))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
