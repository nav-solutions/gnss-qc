use std::collections::HashMap;

use crate::{
    context::QcIndexing,
    prelude::{Constellation, Epoch, SV},
    serializer::data::QcSerializedPreciseState,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QcOrbitProjectionKey {
    pub sv: SV,
    pub epoch: Epoch,
}

#[derive(Clone)]
pub struct QcConstellationOrbitProj {
    pub projection_km: HashMap<QcOrbitProjectionKey, (f64, f64, f64)>,
}

impl QcConstellationOrbitProj {
    #[cfg(feature = "sp3")]
    pub fn from_precise_state(item: QcSerializedPreciseState) -> Self {
        let key = QcOrbitProjectionKey {
            sv: item.data.sv,
            epoch: item.data.epoch,
        };

        let mut projection_km = HashMap::new();
        projection_km.insert(key, item.data.position_km);

        Self { projection_km }
    }

    #[cfg(feature = "sp3")]
    pub fn latch_precise_state(&mut self, item: QcSerializedPreciseState) {
        let key = QcOrbitProjectionKey {
            sv: item.data.sv,
            epoch: item.data.epoch,
        };

        self.projection_km.insert(key, item.data.position_km);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QcOrbitProjectionsKey {
    pub indexing: QcIndexing,
    pub constellation: Constellation,
}

#[derive(Clone)]
pub struct QcOrbitProjections {
    pub projections: HashMap<QcOrbitProjectionsKey, QcConstellationOrbitProj>,
}

impl QcOrbitProjections {
    #[cfg(feature = "sp3")]
    pub fn from_precise_state(item: QcSerializedPreciseState) -> Self {
        let key = QcOrbitProjectionsKey {
            constellation: item.data.sv.constellation,
            indexing: item.indexing.clone(),
        };

        let mut projections = HashMap::new();
        projections.insert(key, QcConstellationOrbitProj::from_precise_state(item));

        Self { projections }
    }

    #[cfg(feature = "sp3")]
    pub fn latch_precise_state(&mut self, item: QcSerializedPreciseState) {
        let key = QcOrbitProjectionsKey {
            constellation: item.data.sv.constellation,
            indexing: item.indexing.clone(),
        };

        if let Some(projection) = self.projections.get_mut(&key) {
            projection.latch_precise_state(item);
        } else {
            self.projections
                .insert(key, QcConstellationOrbitProj::from_precise_state(item));
        }
    }
}
