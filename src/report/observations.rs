use rinex::carrier::Carrier;
use std::collections::HashMap;

use crate::{
    prelude::{Constellation, Epoch, SV},
    report::{sampling::Sampling, temporal_data::TemporalData},
    serializer::signal::{QcSignalDataPoint, QcSignalObservation},
};

pub struct QcConstellationObservationReport {
    /// General sampling condition
    pub sampling: Sampling,

    /// Pseudo Range (meters) per SV and frequency
    pub pseudo_range_m: HashMap<(SV, Carrier), TemporalData>,

    /// Phase Range (meters) per SV and frequency
    pub phase_range_m: HashMap<(SV, Carrier), TemporalData>,

    /// Doppler shifts (Hz/s) per SV and frequency
    pub doppler: HashMap<(SV, Carrier), TemporalData>,
}

impl QcConstellationObservationReport {
    /// Create new [QcConstellationObservationReport]
    pub fn new(observation: QcSignalDataPoint) -> Self {
        let mut sampling = Sampling::default();
        let mut doppler = HashMap::with_capacity(4);
        let mut pseudo_range_m = HashMap::with_capacity(4);
        let mut phase_range_m = HashMap::with_capacity(4);

        match observation.observation {
            QcSignalObservation::Doppler(value) => {
                doppler.insert(
                    (observation.sv, observation.carrier),
                    TemporalData::new(observation.epoch, value),
                );
            }
            QcSignalObservation::PhaseRange(value) => {
                phase_range_m.insert(
                    (observation.sv, observation.carrier),
                    TemporalData::new(observation.epoch, value),
                );
            }
            QcSignalObservation::PseudoRange(value) => {
                pseudo_range_m.insert(
                    (observation.sv, observation.carrier),
                    TemporalData::new(observation.epoch, value),
                );
            }
        }

        sampling.first_epoch = observation.epoch;
        sampling.last_epoch = observation.epoch;
        sampling.total_epochs = 1;

        Self {
            doppler,
            pseudo_range_m,
            phase_range_m,
            sampling,
        }
    }

    /// Latch a new [QcSignalDataPoint]
    pub fn latch_measurement(&mut self, observation: QcSignalDataPoint) {
        match observation.observation {
            QcSignalObservation::Doppler(value) => {
                if let Some(data) = self.doppler.get_mut(&(observation.sv, observation.carrier)) {
                    data.push(observation.epoch, value);
                } else {
                    let data = TemporalData::new(observation.epoch, value);
                    self.doppler
                        .insert((observation.sv, observation.carrier), data);
                }
            }
            QcSignalObservation::PhaseRange(value) => {
                if let Some(data) = self
                    .phase_range_m
                    .get_mut(&(observation.sv, observation.carrier))
                {
                    data.push(observation.epoch, value);
                } else {
                    let data = TemporalData::new(observation.epoch, value);
                    self.phase_range_m
                        .insert((observation.sv, observation.carrier), data);
                }
            }
            QcSignalObservation::PseudoRange(value) => {
                if let Some(data) = self
                    .pseudo_range_m
                    .get_mut(&(observation.sv, observation.carrier))
                {
                    data.push(observation.epoch, value);
                } else {
                    let data = TemporalData::new(observation.epoch, value);
                    self.pseudo_range_m
                        .insert((observation.sv, observation.carrier), data);
                }
            }
        }
    }
}

pub struct QcSignalsObservationReport {
    /// Name of this data source
    pub source: String,

    /// Reported time of first Observation
    pub time_of_first_obs: Option<Epoch>,

    /// Reported time of last Observation
    pub time_of_last_obs: Option<Epoch>,

    /// Report per [Constellation]
    pub constell_report: HashMap<Constellation, QcConstellationObservationReport>,
}

impl QcSignalsObservationReport {
    pub fn new(source: &str) -> Self {
        Self {
            time_of_first_obs: None,
            time_of_last_obs: None,
            source: source.to_string(),
            constell_report: HashMap::with_capacity(4),
        }
    }

    /// Latch a new [QcSignalDataPoint]
    pub fn latch_observation(&mut self, observation: QcSignalDataPoint) {
        if let Some(report) = self.constell_report.get_mut(&observation.sv.constellation) {
            report.latch_measurement(observation);
        } else {
            let constellation = observation.sv.constellation.clone();
            let page = QcConstellationObservationReport::new(observation);
            self.constell_report.insert(constellation, page);
        }
    }
}
