use rinex::carrier::Carrier;
use std::collections::HashMap;

use crate::{
    prelude::{Constellation, Epoch, SV},
    report::{sampling::Sampling, temporal_data::TemporalData},
    serializer::data::{QcSerializedSignal, QcSignalObservation},
};

#[derive(Debug, Clone)]
pub struct QcConstellationObservationsReport {
    /// General sampling condition
    pub sampling: Sampling,

    /// Pseudo Range (meters) per SV and frequency
    pub pseudo_range_m: HashMap<(SV, Carrier), TemporalData>,

    /// Phase Range (meters) per SV and frequency
    pub phase_range_m: HashMap<(SV, Carrier), TemporalData>,

    /// Doppler shifts (Hz/s) per SV and frequency
    pub doppler: HashMap<(SV, Carrier), TemporalData>,
}

impl QcConstellationObservationsReport {
    /// Create new [QcConstellationObservationReport]
    pub fn new(signal: QcSerializedSignal) -> Self {
        let mut sampling = Sampling::default();
        let mut doppler = HashMap::with_capacity(4);
        let mut pseudo_range_m = HashMap::with_capacity(4);
        let mut phase_range_m = HashMap::with_capacity(4);

        match signal.data.observation {
            QcSignalObservation::Doppler(value) => {
                doppler.insert(
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
        }

        sampling.first_epoch = signal.data.epoch;
        sampling.last_epoch = signal.data.epoch;
        sampling.total_epochs = 1;

        Self {
            doppler,
            pseudo_range_m,
            phase_range_m,
            sampling,
        }
    }

    /// Latch a new [QcSignalDataPoint]
    pub fn add_contribution(&mut self, signal: QcSerializedSignal) {
        match signal.data.observation {
            QcSignalObservation::Doppler(value) => {
                if let Some(data) = self.doppler.get_mut(&(signal.data.sv, signal.data.carrier)) {
                    data.push(signal.data.epoch, value);
                } else {
                    let data = TemporalData::new(signal.data.epoch, value);
                    self.doppler
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
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct QcObservationsReport {
    /// Name of this data source
    pub source: String,

    /// Files (contributors)
    pub files: Vec<String>,

    /// Reported time of first Observation
    pub time_of_first_obs: Option<Epoch>,

    /// Reported time of last Observation
    pub time_of_last_obs: Option<Epoch>,

    /// Report per [Constellation]
    pub constellations: HashMap<Constellation, QcConstellationObservationsReport>,
}

impl QcObservationsReport {
    /// Latch a new [QcSignalDataPoint]
    pub fn add_contribution(&mut self, observation: QcSerializedSignal) {
        if self.source.len() == 0 {
            self.source = observation.indexing.to_string();
        }

        self.files.push(observation.filename.to_string());

        if let Some(page) = self
            .constellations
            .get_mut(&observation.data.sv.constellation)
        {
            page.add_contribution(observation);
        } else {
            let constellation = observation.data.sv.constellation.clone();
            let page = QcConstellationObservationsReport::new(observation);
            self.constellations.insert(constellation, page);
        }
    }
}
