use crate::{pipeline::types::QcDataType, serializer::data::QcSerializedItem};


#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum NodeKind {
    QcEphemerisUnwrapper,
    QcObservationsUnwrapper,
    QcObservationsScaler,
}

#[derive(Debug, Clone)]
pub struct Node {
    /// [NodeKind]
    kind: NodeKind,

    /// [Node] name
    pub name: String,

    /// Possible parent name
    pub parent: Option<String>,
}

impl Node {

    pub fn can_process(&self) -> bool {
        true
    }

    pub fn process(&mut self, sample: QcSerializedItem) -> Option<QcSerializedItem> {
        match self.kind {
            NodeKind::QcEphemerisUnwrapper => match sample {
                QcSerializedItem::Qc
            },
            NodeKind::QcObservationsScaler => {

            },
            NodeKind::QcObservationsUnwrapper => {

            },
        }
    }

    pub fn can_connect(&self, rhs: &Self) -> bool {
        match self.kind {
            NodeKind::QcObservationsUnwrapper => match rhs.kind {
                NodeKind::QcObservationsScaler => true,
                _ => false,
            },
            NodeKind::QcObservationsScaler => match rhs.kind {
                NodeKind::QcObservationsScaler => true,
                _ => false,
            },
            NodeKind::QcEphemerisUnwrapper => match rhs.kind {
                _ => false,
            },
        }
    }

    pub fn with_parent(&self, name: &str) -> Self {
        let mut s = self.clone();
        s.parent = Some(name.to_string());
        s
    }

    pub fn observations_unwrapper(name: &str) -> Self {
        Self {
            parent: None,
            name: name.to_string(),
            kind: NodeKind::QcObservationsUnwrapper,
        }
    }

    pub fn observations_scaler(name: &str) -> Self {
        Self {
            parent: None,
            name: name.to_string(),
            kind: NodeKind::QcObservationsScaler,
        }
    }
}