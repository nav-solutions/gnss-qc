use rinex::carrier::Carrier;
use std::collections::HashMap;

use crate::{
    context::QcIndexing,
    prelude::{Constellation, Epoch, SV},
    processing::runner::temporal_data::TemporalData,
    serializer::data::{QcSerializedSignal, QcSignalObservation},
};

pub struct SignalCombination {
    pub sv: SV,
    pub lhs: Carrier,
    pub rhs: Carrier,
    pub indexing: QcIndexing,
    pub value: f64,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SignalKey {
    pub sv: SV,
    pub carrier: Carrier,
    pub indexing: QcIndexing,
}

pub struct SignalsBuffer {
    pub pseudo_range_m: HashMap<SignalKey, (Epoch, f64)>,
    pub carrier_phases_m: HashMap<SignalKey, (Epoch, f64)>,
    pub power_ssi_dbc: HashMap<SignalKey, (Epoch, f64)>,
    pub doppler_shifts_hz_s: HashMap<SignalKey, (Epoch, f64)>,
}

impl SignalsBuffer {
    pub fn new() -> Self {
        Self {
            pseudo_range_m: Default::default(),
            carrier_phases_m: Default::default(),
            doppler_shifts_hz_s: Default::default(),
            power_ssi_dbc: Default::default(),
        }
    }

    pub fn latch(&mut self, signal: &QcSerializedSignal) {
        let key = SignalKey {
            sv: signal.data.sv,
            carrier: signal.data.carrier,
            indexing: signal.indexing.clone(),
        };

        match signal.data.observation {
            QcSignalObservation::Doppler(value) => {
                self.doppler_shifts_hz_z.insert(key, (signal.data.epoch, value));
            },
            QcSignalObservation::PhaseRange(value) => {
                self.carrier_phases_m.insert(key, (signal.data.epoch, value));
            },
            QcSignalObservation::PseudoRange(value) => {
                self.doppler_shifts_hz_s.insert(key, (signal.data.epoch, value));
            },
            QcSignalObservation::SSI(value) => {
                self.power_ssi_dbc.insert(key, (signal.data.epoch, value));
            },
        }
    }

    /// Tries to form a new GF combination
    pub fn phase_gf_combinations(&self, indexing: &QcIndexing, sv: SV, carrier: Carrier, epoch: Epoch) -> Vec<SignalCombination> {
        let mut ret = Vec::<SignalCombination>::new();

        for (k_i, (t_i, v_i)) in self.carrier_phases_m.iter() {
            for (k_j, (t_j, v_j)) in self.carrier_phases_m.iter() {
                if k_i.sv == sv && k_i.carrier == carrier && t_i == epoch && k_i.indexing == indexing {
                    if k_j.indexing == indexing && k_j.sv == sv && k_j.carrier != carrier && t_j == epoch {
                        let f_i = k_i.carrier.frequency();
                        let f_j = k_j.carrier.frequency();

                        let alpha = 1.0 / (f_i.powi(2) - f_j.powi(2));
                        let (beta, gamma) = (f_i.powi(2), f_j.powi(2));

                        ret.push(SignalCombination {
                            sv: k_i.sv,
                            rhs: k_j.carrier,
                            lhs: k_i.carrier,
                            indexing: k_i.indexing.clone(),
                            value: v_i - v_j,
                        });
                    }
                }
            }
        }

        ret
    }
    
}
