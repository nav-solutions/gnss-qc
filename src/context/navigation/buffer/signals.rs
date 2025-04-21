use crate::prelude::QcContext;

use rinex::{
    carrier::Carrier,
    prelude::{Epoch, SV},
};

pub enum QcMeasuredData {
    /// Phase range in meters
    PhaseRange(f64),
    /// Pseudo range in meters
    PseudoRange(f64),
    /// Doppler shift
    DopplerShift(f64),
}

pub struct QcSignalData {
    /// Sampling [Epoch]
    pub t: Epoch,
    /// [SV] signal source
    pub sv: SV,
    /// [Carrier] signal
    pub carrier: Carrier,
    /// [QcMeasuredData]
    pub measurement: QcMeasuredData,
}

pub struct QcSignalBuffer<'a> {
    pub iter: Box<dyn Iterator<Item = QcSignalData> + 'a>,
}

impl QcContext {
    /// Obtain [QcSignalBuffer] from this [QcContext].
    pub fn signals_buffer<'a>(&'a self) -> Option<QcSignalBuffer<'a>> {
        let observations = self.observation()?;

        Some(QcSignalBuffer {
            iter: Box::new(
                observations
                    .signal_observations_sampling_ok_iter()
                    .filter_map(|(t, v)| {
                        match Carrier::from_observable(v.sv.constellation, &v.observable) {
                            Ok(carrier) => {
                                let measurement = if v.observable.is_pseudo_range_observable() {
                                    Some(QcMeasuredData::PseudoRange(v.value))
                                } else if v.observable.is_phase_range_observable() {
                                    Some(QcMeasuredData::PhaseRange(v.value))
                                } else if v.observable.is_doppler_observable() {
                                    Some(QcMeasuredData::DopplerShift(v.value))
                                } else {
                                    None
                                };

                                let measurement = measurement?;

                                Some(QcSignalData {
                                    t,
                                    carrier,
                                    sv: v.sv,
                                    measurement,
                                })
                            }
                            Err(_) => None,
                        }
                    }),
            ),
        })
    }
}
