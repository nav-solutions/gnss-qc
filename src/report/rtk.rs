use std::collections::HashMap;

use crate::{prelude::QcReferencePosition, serializer::data::QcSerializedRINEXHeader};

#[derive(Clone, Default)]
pub struct QcRTKSummary {
    /// Defined Rorvers
    pub rovers: HashMap<String, Option<QcReferencePosition>>,

    /// Define Bases
    pub bases: HashMap<String, Option<QcReferencePosition>>,

    /// Baselines
    pub baselines: HashMap<(String, String), f64>,
}

impl QcRTKSummary {
    pub fn from_rover_header(item: QcSerializedRINEXHeader) -> Self {
        Self {
            rovers: {
                let mut map = HashMap::new();
                if let Some(position) = item.data.rx_position {
                    let position = QcReferencePosition::new(position);
                    map.insert(item.indexing.to_string(), Some(position));
                } else {
                    map.insert(item.indexing.to_string(), None);
                }
                map
            },
            bases: Default::default(),
            baselines: Default::default(),
        }
    }

    pub fn from_base_header(item: QcSerializedRINEXHeader) -> Self {
        Self {
            bases: {
                let mut map = HashMap::new();
                if let Some(position) = item.data.rx_position {
                    let position = QcReferencePosition::new(position);
                    map.insert(item.indexing.to_string(), Some(position));
                } else {
                    map.insert(item.indexing.to_string(), None);
                }
                map
            },
            rovers: Default::default(),
            baselines: Default::default(),
        }
    }

    pub fn latch_rover_header(&mut self, item: QcSerializedRINEXHeader) {
        if let Some(position) = item.data.rx_position {
            let position = QcReferencePosition::new(position);

            self.rovers
                .insert(item.indexing.to_string(), Some(position));
        } else {
            self.rovers.insert(item.indexing.to_string(), None);
        }
    }

    pub fn latch_base_header(&mut self, item: QcSerializedRINEXHeader) {
        if let Some(position) = item.data.rx_position {
            let position = QcReferencePosition::new(position);

            self.bases.insert(item.indexing.to_string(), Some(position));
        } else {
            self.bases.insert(item.indexing.to_string(), None);
        }
    }
}
