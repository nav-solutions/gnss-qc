use crate::{
    context::{QcContext, QcIndexing, QcProductType},
    serializer::{data::QcSerializedEphemeris, iter::QcAbstractIterator},
};

use super::data::QcEphemerisData;

/// [QcEphemerisIterator] used internally to stream data.
pub struct QcEphemerisIterator<'a> {
    /// [QcSynchronousIterator]
    pub iter: QcAbstractIterator<'a, QcSerializedEphemeris<'a>>,
}

impl<'a> Iterator for QcEphemerisIterator<'a> {
    type Item = QcSerializedEphemeris<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl QcContext {
    /// Obtain [QcEphemerisIterator] from current navigation compatible [QcContext]
    /// and desired [QcIndexing] provider.
    pub fn ephemeris_serializer<'a>(
        &'a self,
        indexing: &'a QcIndexing,
    ) -> Option<QcEphemerisIterator<'a>> {
        let (filename, data_set) = self
            .data
            .iter()
            .filter_map(|(k, v)| {
                if k.product_type == QcProductType::BroadcastNavigation && k.indexing == *indexing {
                    Some((&k.filename, v.as_rinex().unwrap()))
                } else {
                    None
                }
            })
            .reduce(|k, _| k)?;

        let iter = data_set
            .nav_ephemeris_frames_iter()
            .filter_map(move |(k, v)| {
                let toe = v.toe(k.sv)?;
                Some(QcSerializedEphemeris {
                    indexing: indexing,
                    filename: filename,
                    product_type: QcProductType::BroadcastNavigation,
                    data: QcEphemerisData {
                        sv: k.sv,
                        toe,
                        toc: k.epoch,
                        ephemeris: v.clone(),
                    },
                })
            });

        Some(QcEphemerisIterator {
            iter: QcAbstractIterator::new(Box::new(iter)),
        })
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::{
        context::QcIndexing,
        prelude::{Epoch, QcContext, SV},
        tests::init_logger,
    };

    #[test]
    fn null_serializer() {
        let mut ctx = QcContext::new();

        // load other type of data
        ctx.load_rinex_file("data/MET/V2/abvi0010.15m").unwrap();

        let marker = QcIndexing::GeodeticMarker("ABVI".to_string());

        assert!(
            ctx.ephemeris_serializer(&marker).is_none(),
            "should not exist!"
        );

        // load NAV
        ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
            .unwrap();

        let marker = QcIndexing::GeodeticMarker("ABVI".to_string());

        assert!(
            ctx.ephemeris_serializer(&marker).is_none(),
            "should not exist!"
        );

        let marker = QcIndexing::None;

        assert!(ctx.ephemeris_serializer(&marker).is_some(), "should exist!");
    }

    #[test]
    fn serializer() {
        init_logger();

        let mut ctx = QcContext::new();

        // load NAV
        ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
            .unwrap();

        let marker = QcIndexing::None;

        let mut serializer = ctx.ephemeris_serializer(&marker).expect("should exist");

        let g01 = SV::from_str("G01").unwrap();

        let t_04_00_00_gpst = Epoch::from_str("2020-06-25T04:00:00 GPST").unwrap();
        let t_06_00_00_gpst = Epoch::from_str("2020-06-25T06:00:00 GPST").unwrap();
        let t_14_00_00_gpst = Epoch::from_str("2020-06-25T14:00:00 GPST").unwrap();
        let t_16_00_00_gpst = Epoch::from_str("2020-06-25T16:00:00 GPST").unwrap();
        let t_18_00_00_gpst = Epoch::from_str("2020-06-25T18:00:00 GPST").unwrap();
        let t_20_00_00_gpst = Epoch::from_str("2020-06-25T20:00:00 GPST").unwrap();

        let mut g01_found = [false, false, false, false, false, false];

        let c19 = SV::from_str("C19").unwrap();

        let t_d0_22_00_00_bdt = Epoch::from_str("2020-06-24T22:00:00 BDT").unwrap();
        let t_d0_23_00_00_bdt = Epoch::from_str("2020-06-24T23:00:00 BDT").unwrap();
        let t_d1_00_00_00_bdt = Epoch::from_str("2020-06-25T00:00:00 BDT").unwrap();
        let t_d1_01_00_00_bdt = Epoch::from_str("2020-06-25T01:00:00 BDT").unwrap();
        let t_d1_02_00_00_bdt = Epoch::from_str("2020-06-25T02:00:00 BDT").unwrap();
        let t_d1_03_00_00_bdt = Epoch::from_str("2020-06-25T03:00:00 BDT").unwrap();
        let t_d1_04_00_00_bdt = Epoch::from_str("2020-06-25T04:00:00 BDT").unwrap();
        let t_d1_10_00_00_bdt = Epoch::from_str("2020-06-25T10:00:00 BDT").unwrap();
        let t_d1_11_00_00_bdt = Epoch::from_str("2020-06-25T11:00:00 BDT").unwrap();
        let t_d1_12_00_00_bdt = Epoch::from_str("2020-06-25T12:00:00 BDT").unwrap();
        let t_d1_13_00_00_bdt = Epoch::from_str("2020-06-25T13:00:00 BDT").unwrap();
        let t_d1_14_00_00_bdt = Epoch::from_str("2020-06-25T14:00:00 BDT").unwrap();

        let mut c19_found = [
            false, false, false, false, false, false, false, false, false, false, false, false,
        ];

        // let s23 = SV::from_str("S23").unwrap();

        // let t_05_36_30_gpst = Epoch::from_str("2020-06-25T05:36:30 GPST").unwrap();
        // let t_05_36_32_gpst = Epoch::from_str("2020-06-25T05:36:32 GPST").unwrap();

        // let mut s23_found = [
        //     false,
        //     false,
        // ];

        // let r24 = SV::from_str("R24").unwrap();

        // let mut r24_found = [
        //     false, false, false, false, false, false, false, false, false, false, false, false,
        //     false, false, false, false, false, false, false, false, false, false, false, false,
        // ];

        // let t_d0_22_45_00_utc = Epoch::from_str("2020-06-24T22:45:00 UTC").unwrap();
        // let t_d0_23_15_00_utc = Epoch::from_str("2020-06-24T23:15:00 UTC").unwrap();
        // let t_d0_23_45_00_utc = Epoch::from_str("2020-06-24T23:45:00 UTC").unwrap();
        // let t_d1_04_45_00_utc = Epoch::from_str("2020-06-25T04:45:00 UTC").unwrap();
        // let t_d1_05_15_00_utc = Epoch::from_str("2020-06-25T05:15:00 UTC").unwrap();
        // let t_d1_05_45_00_utc = Epoch::from_str("2020-06-25T05:45:00 UTC").unwrap();
        // let t_d1_06_15_00_utc = Epoch::from_str("2020-06-25T06:15:00 UTC").unwrap();
        // let t_d1_06_45_00_utc = Epoch::from_str("2020-06-25T06:45:00 UTC").unwrap();
        // let t_d1_07_15_00_utc = Epoch::from_str("2020-06-25T07:15:00 UTC").unwrap();
        // let t_d1_07_45_00_utc = Epoch::from_str("2020-06-25T07:45:00 UTC").unwrap();
        // let t_d1_08_15_00_utc = Epoch::from_str("2020-06-25T08:15:00 UTC").unwrap();
        // let t_d1_08_45_00_utc = Epoch::from_str("2020-06-25T08:45:00 UTC").unwrap();
        // let t_d1_09_15_00_utc = Epoch::from_str("2020-06-25T09:15:00 UTC").unwrap();
        // let t_d1_09_45_00_utc = Epoch::from_str("2020-06-25T09:45:00 UTC").unwrap();
        // let t_d1_18_15_00_utc = Epoch::from_str("2020-06-25T18:15:00 UTC").unwrap();
        // let t_d1_18_45_00_utc = Epoch::from_str("2020-06-25T18:45:00 UTC").unwrap();
        // let t_d1_19_15_00_utc = Epoch::from_str("2020-06-25T19:15:00 UTC").unwrap();
        // let t_d1_19_45_00_utc = Epoch::from_str("2020-06-25T19:45:00 UTC").unwrap();
        // let t_d1_20_15_00_utc = Epoch::from_str("2020-06-25T20:15:00 UTC").unwrap();
        // let t_d1_20_45_00_utc = Epoch::from_str("2020-06-25T20:45:00 UTC").unwrap();
        // let t_d1_21_15_00_utc = Epoch::from_str("2020-06-25T21:15:00 UTC").unwrap();
        // let t_d1_21_45_00_utc = Epoch::from_str("2020-06-25T21:45:00 UTC").unwrap();
        // let t_d1_22_15_00_utc = Epoch::from_str("2020-06-25T22:15:00 UTC").unwrap();
        // let t_d1_22_45_00_utc = Epoch::from_str("2020-06-25T22:45:00 UTC").unwrap();

        let mut points = 0;

        while let Some(serialized) = serializer.next() {
            points += 1;

            if serialized.data.sv == g01 {
                if serialized.data.toc == t_04_00_00_gpst {
                    g01_found[0] = true;
                } else if serialized.data.toc == t_06_00_00_gpst {
                    g01_found[1] = true;
                } else if serialized.data.toc == t_14_00_00_gpst {
                    g01_found[2] = true;
                } else if serialized.data.toc == t_16_00_00_gpst {
                    g01_found[3] = true;
                } else if serialized.data.toc == t_18_00_00_gpst {
                    g01_found[4] = true;
                } else if serialized.data.toc == t_20_00_00_gpst {
                    g01_found[5] = true;
                } else {
                    panic!("found expected G01 data point: {}", serialized.data.toc);
                }
            } else if serialized.data.sv == c19 {
                if serialized.data.toc == t_d0_22_00_00_bdt {
                    c19_found[0] = true;
                } else if serialized.data.toc == t_d0_23_00_00_bdt {
                    c19_found[1] = true;
                } else if serialized.data.toc == t_d1_00_00_00_bdt {
                    c19_found[2] = true;
                } else if serialized.data.toc == t_d1_01_00_00_bdt {
                    c19_found[3] = true;
                } else if serialized.data.toc == t_d1_02_00_00_bdt {
                    c19_found[4] = true;
                } else if serialized.data.toc == t_d1_03_00_00_bdt {
                    c19_found[5] = true;
                } else if serialized.data.toc == t_d1_04_00_00_bdt {
                    c19_found[6] = true;
                } else if serialized.data.toc == t_d1_10_00_00_bdt {
                    c19_found[7] = true;
                } else if serialized.data.toc == t_d1_11_00_00_bdt {
                    c19_found[8] = true;
                } else if serialized.data.toc == t_d1_12_00_00_bdt {
                    c19_found[9] = true;
                } else if serialized.data.toc == t_d1_13_00_00_bdt {
                    c19_found[10] = true;
                } else if serialized.data.toc == t_d1_14_00_00_bdt {
                    c19_found[11] = true;
                } else {
                    panic!("found expected C19 data point: {}", serialized.data.toc);
                }

                // } else if data.sv == s23 {
                //     if data.toc == t_05_36_30_gpst {
                //         s23_found[0] = true;
                //     } else if data.toc == t_05_36_32_gpst {
                //         s23_found[1] = true;
                //     }

                // } else if data.sv == r24 {
                //     if data.toc == t_d0_22_45_00_utc {
                //         r24_found[0] = true;
                //     } else if data.toc == t_d0_23_15_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d0_23_45_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_04_45_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_05_15_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_05_45_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_06_15_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_06_45_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_07_15_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_07_45_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_08_15_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_08_45_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_09_15_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_09_45_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_18_15_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_18_45_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_19_15_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_19_45_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_20_15_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_20_45_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_21_15_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_21_45_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_22_15_00_utc {
                //         r24_found[1] = true;
                //     } else if data.toc == t_d1_22_45_00_utc {
                //         r24_found[1] = true;
                //     } else {
                //         panic!("found expected 24 data point: {}", data.toc);
                //     }
            }
        }

        assert!(points > 0, "nothing streamed by valid data source!!");

        for (index, found) in g01_found.iter().enumerate() {
            assert!(found, "g01 data not found @ {}", index);
        }

        // for (index, found) in r24_found.iter().enumerate() {
        //     assert!(found, "r24 data not found @ {}", index);
        // }

        for (index, found) in c19_found.iter().enumerate() {
            assert!(found, "c19 data not found @ {}", index);
        }

        // for (index, found) in s23_found.iter().enumerate() {
        //     assert!(found, "s23 data not found @ {}", index);
        // }
    }
}
