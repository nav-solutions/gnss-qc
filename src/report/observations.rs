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
    /// True when phase observations should be stored
    pub stores_phase: bool,

    /// True when pseudo range observations should be stored
    pub stores_pseudo_range: bool,

    /// True when doppler shift observations should be stored
    pub stores_dopplers: bool,

    /// True when power observations should be stored
    pub stores_power: bool,

    /// Stored Data
    pub phase_range_m: HashMap<(QcIndexing, Constellation), DataStorage>,

    /// Stored Data
    pub doppler_shifts_hz_s: HashMap<(QcIndexing, Constellation), DataStorage>,

    /// Stored Data
    pub power_ssi_dbc: HashMap<(QcIndexing, Constellation), DataStorage>,

    /// Stored Data
    pub pseudo_range_m: HashMap<(QcIndexing, Constellation), DataStorage>,
}

impl QcObservationsReport {
    /// Initializes a new [QcObservationsReport]
    pub fn new(
        stores_phase: bool,
        stores_dopplers: bool,
        stores_pseudo_range: bool,
        stores_power: bool,
    ) -> Self {
        Self {
            stores_dopplers,
            stores_phase,
            stores_power,
            stores_pseudo_range,
            phase_range_m: Default::default(),
            doppler_shifts_hz_s: Default::default(),
            power_ssi_dbc: Default::default(),
            pseudo_range_m: Default::default(),
        }
    }

    /// Latch new [QcSerializedSignal] contribution
    pub fn latch_signal(&mut self, signal: &QcSerializedSignal) {
        let key = (
            signal.indexing.clone(),
            signal.data.sv.constellation.clone(),
        );

        match signal.data.observation {
            QcSignalObservation::Doppler(value) => {
                if self.stores_dopplers {
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
            }
            QcSignalObservation::PhaseRange(value) => {
                if self.stores_phase {
                    if let Some(inner) = self.phase_range_m.get_mut(&key) {
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
                        self.phase_range_m.insert(key, storage);
                    }
                }
            }
            QcSignalObservation::PseudoRange(value) => {
                if self.stores_pseudo_range {
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
            }
            QcSignalObservation::SSI(value) => {
                if self.stores_power {
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
}
