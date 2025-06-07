use rinex::carrier::Carrier;
use std::collections::HashMap;

use crate::{
    context::QcIndexing,
    prelude::{Constellation, Epoch, SV},
    report::temporal_data::TemporalData,
    serializer::data::{QcSerializedSignal, QcSignalObservation},
};

#[derive(Debug, Clone)]
pub struct QcConstellationObservationsReport {
    /// Pseudo Range (meters) per SV and frequency
    pub pseudo_range_m: HashMap<(SV, Carrier), TemporalData>,

    /// Phase Range (meters) per SV and frequency
    pub phase_range_m: HashMap<(SV, Carrier), TemporalData>,

    /// Doppler shifts (Hz/s) per SV and frequency
    pub doppler_hz_s: HashMap<(SV, Carrier), TemporalData>,

    /// SSI (dBc) per SV and frequency
    pub ssi_dbc: HashMap<(SV, Carrier), TemporalData>,
}

impl QcConstellationObservationsReport {
    /// Create new [QcConstellationObservationReport]
    pub fn new(signal: &QcSerializedSignal) -> Self {
        let mut doppler_hz_s = HashMap::with_capacity(4);
        let mut pseudo_range_m = HashMap::with_capacity(4);
        let mut phase_range_m = HashMap::with_capacity(4);
        let mut ssi_dbc = HashMap::with_capacity(4);

        match signal.data.observation {
            QcSignalObservation::Doppler(value) => {
                doppler_hz_s.insert(
                    (signal.data.sv, signal.data.carrier),
                    TemporalData::new(signal.data.epoch, value),
                );
            }
            QcSignalObservation::PhaseRange(value) => {
                phase_range_m.insert(
                    (signal.data.sv, signal.data.carrier),
                    TemporalData::new(signal.data.epoch, value),
                );
            }
            QcSignalObservation::PseudoRange(value) => {
                pseudo_range_m.insert(
                    (signal.data.sv, signal.data.carrier),
                    TemporalData::new(signal.data.epoch, value),
                );
            }
            QcSignalObservation::SSI(value) => {
                ssi_dbc.insert(
                    (signal.data.sv, signal.data.carrier),
                    TemporalData::new(signal.data.epoch, value),
                );
            }
        }

        Self {
            ssi_dbc,
            doppler_hz_s,
            pseudo_range_m,
            phase_range_m,
        }
    }

    /// Latch a new [QcSignalDataPoint]
    pub fn add_contribution(&mut self, signal: &QcSerializedSignal) {
        match signal.data.observation {
            QcSignalObservation::Doppler(value) => {
                if let Some(data) = self
                    .doppler_hz_s
                    .get_mut(&(signal.data.sv, signal.data.carrier))
                {
                    data.push(signal.data.epoch, value);
                } else {
                    let data = TemporalData::new(signal.data.epoch, value);
                    self.doppler_hz_s
                        .insert((signal.data.sv, signal.data.carrier), data);
                }
            }
            QcSignalObservation::PhaseRange(value) => {
                if let Some(data) = self
                    .phase_range_m
                    .get_mut(&(signal.data.sv, signal.data.carrier))
                {
                    data.push(signal.data.epoch, value);
                } else {
                    let data = TemporalData::new(signal.data.epoch, value);
                    self.phase_range_m
                        .insert((signal.data.sv, signal.data.carrier), data);
                }
            }
            QcSignalObservation::PseudoRange(value) => {
                if let Some(data) = self
                    .pseudo_range_m
                    .get_mut(&(signal.data.sv, signal.data.carrier))
                {
                    data.push(signal.data.epoch, value);
                } else {
                    let data = TemporalData::new(signal.data.epoch, value);
                    self.pseudo_range_m
                        .insert((signal.data.sv, signal.data.carrier), data);
                }
            }
            QcSignalObservation::SSI(value) => {
                if let Some(data) = self.ssi_dbc.get_mut(&(signal.data.sv, signal.data.carrier)) {
                    data.push(signal.data.epoch, value);
                } else {
                    let data = TemporalData::new(signal.data.epoch, value);
                    self.ssi_dbc
                        .insert((signal.data.sv, signal.data.carrier), data);
                }
            }
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct QcObservationsReport {
    /// Reported time of first Observation.
    pub time_of_first_obs: Option<Epoch>,

    /// Reported time of last Observation.
    pub time_of_last_obs: Option<Epoch>,

    /// Report Data
    pub data: HashMap<(QcIndexing, Constellation), QcConstellationObservationsReport>,
}

impl QcObservationsReport {
    /// Latch a new [QcSignalDataPoint]
    pub fn add_contribution(&mut self, observation: &QcSerializedSignal) {
        let key = (
            observation.indexing.clone(),
            observation.data.sv.constellation.clone(),
        );

        if let Some(time_of_first_obs) = &mut self.time_of_first_obs {
            if observation.data.epoch < *time_of_first_obs {
                *time_of_first_obs = observation.data.epoch;
            }
        } else {
            self.time_of_first_obs = Some(observation.data.epoch);
        }

        if let Some(time_of_last_obs) = &mut self.time_of_last_obs {
            if observation.data.epoch > *time_of_last_obs {
                *time_of_last_obs = observation.data.epoch;
            }
        } else {
            self.time_of_last_obs = Some(observation.data.epoch);
        }

        if let Some(page) = self.data.get_mut(&key) {
            page.add_contribution(observation);
        } else {
            let page = QcConstellationObservationsReport::new(observation);
            self.data.insert(key.clone(), page);
        }
    }
}
