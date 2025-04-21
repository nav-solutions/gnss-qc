use log::error;

use crate::prelude::{Error, QcContext};

use rinex::{
    carrier::Carrier,
    prelude::{Epoch, SV},
};

use gnss_rtk::prelude::{Carrier as RTKCarrier, Observation as RTKObservation};

#[derive(Clone)]
pub enum QcMeasuredData {
    /// Phase range in meters
    PhaseRange(f64),
    /// Pseudo range in meters
    PseudoRange(f64),
    /// Doppler shift
    DopplerShift(f64),
}

#[derive(Clone)]
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

impl QcSignalData {
    /// Convert [QcSignalData] to RTK compatible [Observation]
    pub fn to_observation(&self) -> Result<RTKObservation, Error> {
        let carrier = RTKCarrier::from_frequency_mega_hz(self.carrier.frequency_mega_hz())
            .map_err(|_| Error::NonSupportedSignal)?;

        match self.measurement {
            QcMeasuredData::PhaseRange(value) => {
                Ok(RTKObservation::ambiguous_phase_range(carrier, value, None))
            }
            QcMeasuredData::PseudoRange(value) => {
                Ok(RTKObservation::pseudo_range(carrier, value, None))
            }
            QcMeasuredData::DopplerShift(value) => {
                Ok(RTKObservation::doppler(carrier, value, None))
            }
        }
    }
}

pub struct QcSignalBuffer<'a> {
    pub iter: Box<dyn Iterator<Item = QcSignalData> + 'a>,
}

impl<'a> Iterator for QcSignalBuffer<'a> {
    type Item = QcSignalData;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
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
                            Err(e) => {
                                error!(
                                    "{}({}/{}) - non supported signal {:?}",
                                    t, v.sv.constellation, v.observable, e,
                                );

                                None
                            }
                        }
                    }),
            ),
        })
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::{
        context::navigation::buffer::signals::QcMeasuredData,
        prelude::{Epoch, QcContext, SV},
    };

    use rinex::carrier::Carrier;

    #[test]
    fn signal_buffering() {
        let mut ctx = QcContext::new();

        // load other type of data
        ctx.load_rinex_file("data/MET/V2/abvi0010.15m").unwrap();

        assert!(ctx.signals_buffer().is_none(), "non existing signals!");

        // load observations
        ctx.load_rinex_file("data/OBS/V3/LARM0010.22O").unwrap();

        let mut signals = ctx.signals_buffer().expect("signals buffer should exist!");

        let g01 = SV::from_str("G01").unwrap();
        let r24 = SV::from_str("R24").unwrap();

        let t0 = Epoch::from_str("2022-01-01T00:00:00 GPST").unwrap();
        let t1 = Epoch::from_str("2022-01-01T00:00:30 GPST").unwrap();
        let t2 = Epoch::from_str("2022-01-01T00:01:00 GPST").unwrap();
        let t3 = Epoch::from_str("2022-01-01T00:01:30 GPST").unwrap();

        let (mut t0_g01_found, mut t1_g01_found, mut t2_g01_found, mut t3_g01_found) =
            (false, false, false, false);

        let (mut t0_r24_found, mut t1_r24_found, mut t2_r24_found, mut t3_r24_found) =
            (false, false, false, false);

        while let Some(signal) = signals.next() {
            if signal.t == t0 {
                if signal.sv == g01 {
                    match (signal.carrier, signal.measurement) {
                        (Carrier::L1, QcMeasuredData::PseudoRange(_)) => {
                            if t0_g01_found {
                                panic!("Iterator proposed duplicated sample: {}/{}", t0, g01);
                            }
                            t0_g01_found = true;
                        }
                        _ => {} // not tested
                    }
                } else if signal.sv == r24 {
                    match (signal.carrier, signal.measurement) {
                        (Carrier::G1(_), QcMeasuredData::PseudoRange(_)) => {
                            if t0_r24_found {
                                panic!("Iterator proposed duplicated sample: {}/{}", t0, r24);
                            }
                            t0_r24_found = true;
                        }
                        _ => {} // not tested
                    }
                } else {
                    // not tested
                }
            } else if signal.t == t1 {
                if signal.sv == g01 {
                    match (signal.carrier, signal.measurement) {
                        (Carrier::L1, QcMeasuredData::PseudoRange(_)) => {
                            if t1_g01_found {
                                panic!("Iterator proposed duplicated sample: {}/{}", t1, g01);
                            }
                            t1_g01_found = true;
                        }
                        _ => {} // not tested
                    }
                } else if signal.sv == r24 {
                    match (signal.carrier, signal.measurement) {
                        (Carrier::G1(_), QcMeasuredData::PseudoRange(_)) => {
                            if t1_r24_found {
                                panic!("Iterator proposed duplicated sample: {}/{}", t1, r24);
                            }
                            t1_r24_found = true;
                        }
                        _ => {} // not tested
                    }
                } else {
                    // not tested
                }
            } else if signal.t == t2 {
                if signal.sv == g01 {
                    match (signal.carrier, signal.measurement) {
                        (Carrier::L1, QcMeasuredData::PseudoRange(_)) => {
                            if t2_g01_found {
                                panic!("Iterator proposed duplicated sample: {}/{}", t2, g01);
                            }
                            t2_g01_found = true;
                        }
                        _ => {} // not tested
                    }
                } else if signal.sv == r24 {
                    match (signal.carrier, signal.measurement) {
                        (Carrier::G1(_), QcMeasuredData::PseudoRange(_)) => {
                            if t2_r24_found {
                                panic!("Iterator proposed duplicated sample: {}/{}", t2, r24);
                            }
                            t2_r24_found = true;
                        }
                        _ => {} // not tested
                    }
                } else {
                    // not tested
                }
            } else if signal.t == t3 {
                if signal.sv == g01 {
                    match (signal.carrier, signal.measurement) {
                        (Carrier::L1, QcMeasuredData::PseudoRange(_)) => {
                            if t3_g01_found {
                                panic!("Iterator proposed duplicated sample: {}/{}", t3, g01);
                            }
                            t3_g01_found = true;
                        }
                        _ => {} // not tested
                    }
                } else if signal.sv == r24 {
                    match (signal.carrier, signal.measurement) {
                        (Carrier::G1(_), QcMeasuredData::PseudoRange(_)) => {
                            if t3_r24_found {
                                panic!("Iterator proposed duplicated sample: {}/{}", t3, r24);
                            }
                            t3_r24_found = true;
                        }
                        _ => {} // not tested
                    }
                } else {
                    // not tested
                }
            } else {
                panic!("Iterator proposed incorrect {} epoch!", signal.t);
            }
        }

        assert!(t0_g01_found, "T0/G01 data is missing!");
        assert!(t1_g01_found, "T1/G01 data is missing!");
        assert!(t2_g01_found, "T2/G01 data is missing!");
        assert!(t3_g01_found, "T3/G01 data is missing!");

        assert!(t0_r24_found, "T0/R24 data is missing!");
        assert!(t1_r24_found, "T1/R24 data is missing!");
        assert!(t2_r24_found, "T2/R24 data is missing!");
        assert!(t3_r24_found, "T3/R24 data is missing!");
    }
}
