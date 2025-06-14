use rinex::carrier::Carrier;
use std::collections::HashMap;

use crate::{
    context::QcIndexing,
    prelude::{Constellation, Epoch, SV},
    serializer::data::{QcSerializedSignal, QcSignalObservation},
};

pub(crate) struct DataStorage {
    /// Reported time of first Observation.
    pub time_of_first_obs: Epoch,

    /// Reported time of last Observation.
    pub time_of_last_obs: Epoch,

    /// Inernal data per SV and frequency
    pub storage: HashMap<(SV, Carrier), TemporalData>,
}

impl DataStorage {
    /// Create new [DataStorage]
    pub fn new(sv: SV, carrier: Carrier, epoch: Epoch, point: f64) -> Self {
        let mut storage = HashMap::with_capacity(4);
        storage.insert((sv, carrier), TemporalData::new(epoch, point));

        Self {
            storage,
            time_of_first_obs: epoch,
            time_of_last_obs: epoch,
        }
    }

    /// Latch a new data point
    pub fn latch_data_point(&mut self, sv: SV, carrier: Carrier, epoch: Epoch, point: f64) {
        self.time_of_last_obs = epoch;

        if let Some(storage) = self.storage.get_mut(&(sv, carrier)) {
            storage.push(epoch, point);
        } else {
            self.storage
                .insert((sv, carrier), TemporalData::new(epoch, point));
        }
    }
}

pub(crate) struct QcObservationsReport {
    /// name
    /// report name
    pub name: String,

    /// Overall time of first observation
    pub time_of_first_obs: Epoch,
    
    /// Overall time of last observation
    pub time_of_last_obs: Epoch,

    /// Stored Data
    pub data: HashMap<(QcIndexing, Constellation), DataStorage>,
}

impl QcObservationsReport {
    /// Initializes a new [QcObservationsReport]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            data: Default::default(),
            time_of_last_obs: Default::default(),
            time_of_first_obs: Default::default(),
        }
    }

    /// Latch new [QcSerializedSignal] contribution
    pub fn latch_data_point(&mut self, indexing: &QcIndexing, sv: SV, carrier: Carrier, epoch: Epoch, value: f64) {
        let key = (
            indexing: indexing.clone(),
            data.sv.constellation.clone(),
        );

        if epoch < self.time_of_first_obs {
            self.time_of_first_obs = epoch;
        }

        if epoch > self.time_of_last_obs {
            self.time_of_last_obs = epoch;
        }

        if let Some(inner) = self.data.get_mut(&key) {
            data.latch_data_point(
                sv,
                carrier,
                epoch,
                value,
            );
        } else {
            let storage = DataStorage::new(
                sv,
                carrier,
                epoch,
                value,
            );
            self.data.insert(key, storage);
        }
    }
}
