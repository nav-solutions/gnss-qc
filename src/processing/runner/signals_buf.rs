use rinex::carrier::Carrier;
use std::collections::HashMap;

use crate::{
    context::QcIndexing,
    prelude::{Constellation, Epoch, SV},
    processing::runner::temporal_data::TemporalData,
    serializer::data::{QcSerializedSignal, QcSignalObservation},
};

pub struct DataStorage {
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

pub struct SignalsBuffer {
    pub time_of_first_obs: Epoch,
    pub time_of_last_obs: Epoch,
    pub pseudo_range_m: HashMap<(QcIndexing, Constellation), DataStorage>,
    pub carrier_phases_m: HashMap<(QcIndexing, Constellation), DataStorage>,
    pub doppler_shifts_hz_s: HashMap<(QcIndexing, Constellation), DataStorage>,
    pub power_ssi_dbc: HashMap<(QcIndexing, Constellation), DataStorage>,
}

impl SignalsBuffer {
    pub fn new() -> Self {
        Self {
            time_of_first_obs: Default::default(),
            time_of_last_obs: Default::default(),
            pseudo_range_m: Default::default(),
            carrier_phases_m: Default::default(),
            doppler_shifts_hz_s: Default::default(),
            power_ssi_dbc: Default::default(),
        }
    }

    pub fn latch(&mut self, signal: &QcSerializedSignal) {
        let key = (
            signal.indexing.clone(),
            signal.data.sv.constellation.clone(),
        );

        match signal.data.observation {
            QcSignalObservation::Doppler(value) => {
                if let Some(inner) = self.doppler_shifts_hz_s.get_mut(&key) {
                    inner.latch_data_point(
                        signal.data.sv,
                        signal.data.carrier,
                        signal.data.epoch,
                        value,
                    );
                } else {
                    let storage = DataStorage::new(
                        signal.data.sv,
                        signal.data.carrier,
                        signal.data.epoch,
                        value,
                    );
                    self.doppler_shifts_hz_s.insert(key, storage);
                }
            }
            QcSignalObservation::PhaseRange(value) => {
                if let Some(inner) = self.carrier_phases_m.get_mut(&key) {
                    inner.latch_data_point(
                        signal.data.sv,
                        signal.data.carrier,
                        signal.data.epoch,
                        value,
                    );
                } else {
                    let storage = DataStorage::new(
                        signal.data.sv,
                        signal.data.carrier,
                        signal.data.epoch,
                        value,
                    );
                    self.carrier_phases_m.insert(key, storage);
                }
            }
            QcSignalObservation::PseudoRange(value) => {
                if let Some(inner) = self.pseudo_range_m.get_mut(&key) {
                    inner.latch_data_point(
                        signal.data.sv,
                        signal.data.carrier,
                        signal.data.epoch,
                        value,
                    );
                } else {
                    let storage = DataStorage::new(
                        signal.data.sv,
                        signal.data.carrier,
                        signal.data.epoch,
                        value,
                    );
                    self.pseudo_range_m.insert(key, storage);
                }
            }
            QcSignalObservation::SSI(value) => {
                if let Some(inner) = self.power_ssi_dbc.get_mut(&key) {
                    inner.latch_data_point(
                        signal.data.sv,
                        signal.data.carrier,
                        signal.data.epoch,
                        value,
                    );
                } else {
                    let storage = DataStorage::new(
                        signal.data.sv,
                        signal.data.carrier,
                        signal.data.epoch,
                        value,
                    );
                    self.power_ssi_dbc.insert(key, storage);
                }
            }
        }
    }
}
