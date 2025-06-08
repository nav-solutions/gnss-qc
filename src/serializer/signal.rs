use crate::{
    context::{QcContext, QcIndexing, QcProductType},
    serializer::{
        data::{QcSerializedSignal, QcSignalObservation},
        iter::QcAbstractIterator,
    },
};

use rinex::{carrier::Carrier, prelude::Observable};

use super::data::QcSignalData;

pub struct QcSignalIterator<'a> {
    /// [QcSynchronousIterator]
    pub iter: QcAbstractIterator<'a, QcSerializedSignal<'a>>,
}

impl QcContext {
    /// Obtain [QcSignalSerializer] scoped to [QcIndexing] data source, from current [QcContext].
    pub fn signal_serializer<'a>(
        &'a self,
        indexing: &'a QcIndexing,
    ) -> Option<QcSignalIterator<'a>> {
        let (filename, data_set) = self
            .data
            .iter()
            .filter_map(|(k, v)| {
                if k.product_type == QcProductType::Observation && k.indexing == *indexing {
                    Some((&k.filename, v.as_rinex().unwrap()))
                } else {
                    None
                }
            })
            .reduce(|k, _| k)?;

        let iter = data_set
            .signal_observations_iter()
            .filter_map(move |(k, v)| {
                if let Ok(carrier) = Carrier::from_observable(v.sv.constellation, &v.observable) {
                    let observation = match &v.observable {
                        Observable::Doppler(_) => Some(QcSignalObservation::Doppler(v.value)),
                        Observable::PhaseRange(_) => Some(QcSignalObservation::PhaseRange(v.value)),
                        Observable::PseudoRange(_) => {
                            Some(QcSignalObservation::PseudoRange(v.value))
                        }
                        observable => {
                            trace!("{}({}) - unhanled observable {}", k.epoch, v.sv, observable);
                            None
                        }
                    };

                    let observation = observation?;

                    Some(QcSerializedSignal {
                        indexing: indexing,
                        filename: filename,
                        product_type: QcProductType::Observation,
                        data: QcSignalData {
                            epoch: k.epoch,
                            carrier,
                            sv: v.sv,
                            observation,
                            lli: v.lli,
                        },
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
            });

        Some(QcSignalIterator {
            iter: QcAbstractIterator::new(Box::new(iter)),
        })
    }
}

impl<'a> Iterator for QcSignalIterator<'a> {
    type Item = QcSerializedSignal<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::{
        context::QcIndexing,
        prelude::{Epoch, QcContext, SV},
        serializer::signal::QcSignalObservation,
        tests::init_logger,
    };

    use rinex::carrier::Carrier;

    #[test]
    fn null_serializer() {
        let mut ctx = QcContext::new();

        // load other type of data
        ctx.load_rinex_file("data/MET/V2/abvi0010.15m").unwrap();

        let agency = "test".to_string();
        let source = QcIndexing::Agency(agency);

        assert!(ctx.signal_serializer(&source).is_none(), "should not exist");
    }

    #[test]
    fn serializer() {
        init_logger();

        let mut ctx = QcContext::new();

        // load data
        ctx.load_rinex_file("data/OBS/V3/VLNS0010.22O").unwrap();

        let marker = "VLNS-10801M001".to_string();
        let source = QcIndexing::GeodeticMarker(marker);

        let mut serializer = ctx.signal_serializer(&source).expect("should exist");

        let g08 = SV::from_str("G08").unwrap();

        let t_00_00_00_gpst = Epoch::from_str("2022-01-01T00:00:00 GPST").unwrap();
        let t_00_00_30_gpst = Epoch::from_str("2022-01-01T00:00:30 GPST").unwrap();
        let t_00_01_00_gpst = Epoch::from_str("2022-01-01T00:01:00 GPST").unwrap();

        let mut points = 0;

        let mut g08_c1c_found = [false, false, false];
        let mut g08_l1c_found = [false, false, false];
        let mut g08_c2c_found = [false, false, false];
        let mut g08_l2c_found = [false, false, false];

        while let Some(serialized) = serializer.next() {
            points += 1;

            if serialized.data.sv == g08 {
                if serialized.data.epoch == t_00_00_00_gpst {
                    if serialized.data.carrier == Carrier::L1 {
                        match serialized.data.observation {
                            QcSignalObservation::PseudoRange(value) => {
                                assert_eq!(value, 20982937.082);
                                assert!(
                                    serialized.data.lli.is_none(),
                                    "proposed non existing flags!"
                                );
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
                            QcSignalObservation::SSI(_) => {}
                        }
                    } else if serialized.data.carrier == Carrier::L2 {
                        match serialized.data.observation {
                            QcSignalObservation::PseudoRange(value) => {
                                assert_eq!(value, 20982932.182);
                                assert!(
                                    serialized.data.lli.is_none(),
                                    "proposed non existing flags!"
                                );
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
                            QcSignalObservation::SSI(_) => {}
                        }
                    } else {
                        panic!(
                            "Found invalid {} carrier frequency for G08",
                            serialized.data.carrier
                        )
                    }
                } else if serialized.data.epoch == t_00_00_30_gpst {
                    if serialized.data.carrier == Carrier::L1 {
                        match serialized.data.observation {
                            QcSignalObservation::PseudoRange(value) => {
                                assert_eq!(value, 20975946.902);
                                assert!(
                                    serialized.data.lli.is_none(),
                                    "proposed non existing flags!"
                                );
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
                            QcSignalObservation::SSI(_) => {}
                        }
                    } else if serialized.data.carrier == Carrier::L2 {
                        match serialized.data.observation {
                            QcSignalObservation::PseudoRange(value) => {
                                assert_eq!(value, 20975942.022);
                                assert!(
                                    serialized.data.lli.is_none(),
                                    "proposed non existing flags!"
                                );
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
                            QcSignalObservation::SSI(_) => {}
                        }
                    } else {
                        panic!(
                            "Found invalid {} carrier frequency for G08",
                            serialized.data.carrier
                        )
                    }
                } else if serialized.data.epoch == t_00_01_00_gpst {
                    if serialized.data.carrier == Carrier::L1 {
                        match serialized.data.observation {
                            QcSignalObservation::PseudoRange(value) => {
                                assert_eq!(value, 20969053.982);
                                assert!(
                                    serialized.data.lli.is_none(),
                                    "proposed non existing flags!"
                                );
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
                            QcSignalObservation::SSI(_) => {}
                        }
                    } else if serialized.data.carrier == Carrier::L2 {
                        match serialized.data.observation {
                            QcSignalObservation::PseudoRange(value) => {
                                assert_eq!(value, 20969049.082);
                                assert!(
                                    serialized.data.lli.is_none(),
                                    "proposed non existing flags!"
                                );
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
                            QcSignalObservation::SSI(_) => {}
                        }
                    } else {
                        panic!(
                            "Found invalid {} carrier frequency for G08",
                            serialized.data.carrier
                        )
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
