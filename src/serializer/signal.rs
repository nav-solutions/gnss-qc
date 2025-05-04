use rinex::{
    observation::LliFlags,
    prelude::{Carrier, Observable},
};

use crate::{
    context::{QcContext, QcIndexing},
    prelude::{Epoch, SV},
    serializer::sync::QcSynchronousIterator,
};

use log::trace;

#[cfg(doc)]
use super::serializer::QcSerializer;

#[derive(Debug, Clone)]
pub enum QcSignalObservation {
    /// Pseudo Range in meters
    PseudoRange(f64),

    /// Phase Range in meters
    PhaseRange(f64),

    /// Doppler shift in Hz/s
    Doppler(f64),
}

/// [QcSignalDataPoint] streamed by [QcSerializer]
#[derive(Debug, Clone)]
pub struct QcSignalDataPoint {
    /// Sampling [Epoch]
    pub epoch: Epoch,

    /// Readable name of data provider
    pub name: String,

    /// [SV] signal source
    pub sv: SV,

    /// Possibly attached flags
    pub lli: Option<LliFlags>,

    /// [Carrier] frequency
    pub carrier: Carrier,

    /// [QcSignalObservation]
    pub observation: QcSignalObservation,
}

pub struct QcSignalSerializer<'a> {
    /// [QcSynchronousIterator]
    buffered_iter: QcSynchronousIterator<'a, QcSignalDataPoint>,
}

impl QcContext {
    /// Obtain [QcSignalSerializer] scoped to [QcIndexing] data source, from current [QcContext].
    pub fn signal_serializer<'a>(&'a self, source: &QcIndexing) -> QcSignalSerializer<'a> {
        let name = source.to_string();

        let buffered_iter = match self
            .observations
            .iter()
            .filter_map(|(k, v)| if k == source { Some(v) } else { None })
            .reduce(|k, _| k)
        {
            Some(v) => QcSynchronousIterator::new(Box::new(
                v.signal_observations_iter().filter_map(move |(k, v)| {
                    if let Ok(carrier) = Carrier::from_observable(v.sv.constellation, &v.observable)
                    {
                        let observation = match &v.observable {
                            Observable::Doppler(_) => Some(QcSignalObservation::Doppler(v.value)),
                            Observable::PhaseRange(_) => {
                                Some(QcSignalObservation::PhaseRange(v.value))
                            }
                            Observable::PseudoRange(_) => {
                                Some(QcSignalObservation::PseudoRange(v.value))
                            }
                            observable => {
                                trace!(
                                    "{}({}) - unhanled observable {}",
                                    k.epoch,
                                    v.sv,
                                    observable
                                );
                                None
                            }
                        };

                        let observation = observation?;

                        Some(QcSignalDataPoint {
                            name: name.clone(),
                            epoch: k.epoch,
                            carrier,
                            sv: v.sv,
                            observation,
                            lli: v.lli,
                        })
                    } else {
                        trace!(
                            "{}({}) - non supported frequency: {}",
                            k.epoch,
                            v.sv,
                            v.observable
                        );
                        None
                    }
                }),
            )),
            None => QcSynchronousIterator::null(),
        };

        QcSignalSerializer { buffered_iter }
    }
}

impl<'a> Iterator for QcSignalSerializer<'a> {
    type Item = QcSignalDataPoint;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffered_iter.next()
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::{
        context::QcIndexing,
        prelude::{Epoch, QcContext, SV},
        serializer::signal::QcSignalObservation,
    };

    use rinex::carrier::Carrier;

    #[test]
    fn null_serializer() {
        let mut ctx = QcContext::new();

        // load other type of data
        ctx.load_rinex_file("data/MET/V2/abvi0010.15m").unwrap();

        let agency = "test".to_string();
        let source = QcIndexing::Agency(agency);

        let mut serializer = ctx.signal_serializer(&source);

        let mut points = 0;
        while let Some(_) = serializer.next() {
            points += 1;
        }

        assert_eq!(points, 0, "Found data points for unexisting agency!");
    }

    #[test]
    fn serializer() {
        let mut ctx = QcContext::new();

        // load data
        ctx.load_rinex_file("data/OBS/V3/VLNS0010.22O").unwrap();

        let marker = "VLNS-10801M001".to_string();
        let source = QcIndexing::GeodeticMarker(marker);

        let mut serializer = ctx.signal_serializer(&source);

        let g08 = SV::from_str("G08").unwrap();

        let t_00_00_00_gpst = Epoch::from_str("2022-01-01T00:00:00 GPST").unwrap();
        let t_00_00_30_gpst = Epoch::from_str("2022-01-01T00:00:30 GPST").unwrap();
        let t_00_01_00_gpst = Epoch::from_str("2022-01-01T00:01:00 GPST").unwrap();

        let mut points = 0;

        let mut g08_c1c_found = [false, false, false];
        let mut g08_l1c_found = [false, false, false];
        let mut g08_c2c_found = [false, false, false];
        let mut g08_l2c_found = [false, false, false];

        while let Some(data) = serializer.next() {
            points += 1;

            if data.sv == g08 {
                if data.epoch == t_00_00_00_gpst {
                    if data.carrier == Carrier::L1 {
                        match data.observation {
                            QcSignalObservation::PseudoRange(value) => {
                                assert_eq!(value, 20982937.082);
                                assert!(data.lli.is_none(), "proposed non existing flags!");
                                g08_c1c_found[0] = true;
                            }
                            QcSignalObservation::PhaseRange(value) => {
                                assert_eq!(value, 110266080.971);
                                // assert!(data.lli.is_some(), "phase flags dropped!");
                                g08_l1c_found[0] = true;
                            }
                            QcSignalObservation::Doppler(_) => {
                                panic!("this file does not have dopplers!");
                            }
                        }
                    } else if data.carrier == Carrier::L2 {
                        match data.observation {
                            QcSignalObservation::PseudoRange(value) => {
                                assert_eq!(value, 20982932.182);
                                assert!(data.lli.is_none(), "proposed non existing flags!");
                                g08_c2c_found[0] = true;
                            }
                            QcSignalObservation::PhaseRange(value) => {
                                assert_eq!(value, 85921597.759);
                                // assert!(data.lli.is_some(), "phase flags dropped!");
                                g08_l2c_found[0] = true;
                            }
                            QcSignalObservation::Doppler(_) => {
                                panic!("this file does not have dopplers!");
                            }
                        }
                    } else {
                        panic!("Found invalid {} carrier frequency for G08", data.carrier)
                    }
                } else if data.epoch == t_00_00_30_gpst {
                    if data.carrier == Carrier::L1 {
                        match data.observation {
                            QcSignalObservation::PseudoRange(value) => {
                                assert_eq!(value, 20975946.902);
                                assert!(data.lli.is_none(), "proposed non existing flags!");
                                g08_c1c_found[1] = true;
                            }
                            QcSignalObservation::PhaseRange(value) => {
                                assert_eq!(value, 110229347.351);
                                // assert!(data.lli.is_some(), "phase flags dropped!");
                                g08_l1c_found[1] = true;
                            }
                            QcSignalObservation::Doppler(_) => {
                                panic!("this file does not have dopplers!");
                            }
                        }
                    } else if data.carrier == Carrier::L2 {
                        match data.observation {
                            QcSignalObservation::PseudoRange(value) => {
                                assert_eq!(value, 20975942.022);
                                assert!(data.lli.is_none(), "proposed non existing flags!");
                                g08_c2c_found[1] = true;
                            }
                            QcSignalObservation::PhaseRange(value) => {
                                assert_eq!(value, 85892974.149);
                                // assert!(data.lli.is_some(), "phase flags dropped!");
                                g08_l2c_found[1] = true;
                            }
                            QcSignalObservation::Doppler(_) => {
                                panic!("this file does not have dopplers!");
                            }
                        }
                    } else {
                        panic!("Found invalid {} carrier frequency for G08", data.carrier)
                    }
                } else if data.epoch == t_00_01_00_gpst {
                    if data.carrier == Carrier::L1 {
                        match data.observation {
                            QcSignalObservation::PseudoRange(value) => {
                                assert_eq!(value, 20969053.982);
                                assert!(data.lli.is_none(), "proposed non existing flags!");
                                g08_c1c_found[2] = true;
                            }
                            QcSignalObservation::PhaseRange(value) => {
                                assert_eq!(value, 110193124.798);
                                // assert!(data.lli.is_some(), "phase flags dropped!");
                                g08_l1c_found[2] = true;
                            }
                            QcSignalObservation::Doppler(_) => {
                                panic!("this file does not have dopplers!");
                            }
                        }
                    } else if data.carrier == Carrier::L2 {
                        match data.observation {
                            QcSignalObservation::PseudoRange(value) => {
                                assert_eq!(value, 20969049.082);
                                assert!(data.lli.is_none(), "proposed non existing flags!");
                                g08_c2c_found[2] = true;
                            }
                            QcSignalObservation::PhaseRange(value) => {
                                assert_eq!(value, 85864748.777);
                                // assert!(data.lli.is_some(), "phase flags dropped!");
                                g08_l2c_found[2] = true;
                            }
                            QcSignalObservation::Doppler(_) => {
                                panic!("this file does not have dopplers!");
                            }
                        }
                    } else {
                        panic!("Found invalid {} carrier frequency for G08", data.carrier)
                    }
                }
            }
        }

        assert!(points > 0, "nothing streamed by valid data source!!");

        for (index, found) in g08_c1c_found.iter().enumerate() {
            assert!(found, "G08 C1C missing @ {}", index);
        }
        for (index, found) in g08_l1c_found.iter().enumerate() {
            assert!(found, "G08 L1C missing @ {}", index);
        }
        for (index, found) in g08_c2c_found.iter().enumerate() {
            assert!(found, "G08 C2C missing @ {}", index);
        }
        for (index, found) in g08_l2c_found.iter().enumerate() {
            assert!(found, "G08 L2C missing @ {}", index);
        }
    }
}
