use crate::prelude::QcContext;

use rinex::{
    navigation::Ephemeris,
    prelude::{Duration, Epoch, SV},
};

use gnss_rtk::prelude::{Frame, Orbit, OrbitSource};

pub struct QcEphemerisData {
    pub sv: SV,
    pub toe: Epoch,
    pub toc: Epoch,
    pub ephemeris: Ephemeris,
}

impl QcEphemerisData {
    /// Converts [QcEphemerisData] to ANISE [Orbit]
    fn to_orbit(&self, t: Epoch) -> Option<Orbit> {
        let orbit = self.ephemeris.kepler2position(self.sv, self.toc, t)?;
        debug!("{}({}) - keplerian state: {}", t, self.sv, orbit);
        Some(orbit)
    }
}

/// [QcEphemerisBuffer] is constructed from a [QcContext] and used to
/// Iterator the data set in any post navigation process.
pub struct QcEphemerisBuffer<'a> {
    /// Reference [Frame] used in navigation process.
    /// Should be an Earth Centered [Frame] for 100% correctness.
    frame: Frame,

    /// [QcEphemerisData] Iterator
    pub iter: Box<dyn Iterator<Item = QcEphemerisData> + 'a>,

    /// Buffered [QcEphemerisData]
    buffered: Vec<QcEphemerisData>,
}

impl<'a> QcEphemerisBuffer<'a> {
    pub fn group_delay(&mut self, sv: SV, t: Epoch) -> Option<Duration> {
        // discard outdated
        self.buffered
            .retain(|k| k.ephemeris.is_valid(k.sv, t, k.toe));

        // gather new data
        loop {
            if let Some(next) = self.iter.next() {
                if next.sv == sv && !next.ephemeris.is_valid(sv, t, next.toe) {
                    self.buffered.push(next);
                    break;
                } else {
                    self.buffered.push(next);
                }
            } else {
                break;
            }
        }

        let buffered = self
            .buffered
            .iter()
            .filter(|k| k.ephemeris.is_valid(sv, t, k.toe))
            .min_by_key(|k| k.toe - t)?;

        buffered.ephemeris.tgd()
    }
}

impl<'a> OrbitSource for QcEphemerisBuffer<'a> {
    fn next_at(&mut self, t: Epoch, sv: SV, _: Frame) -> Option<Orbit> {
        // discard outdated
        self.buffered
            .retain(|k| k.ephemeris.is_valid(k.sv, t, k.toe));

        // gather new data
        loop {
            if let Some(next) = self.iter.next() {
                if next.sv == sv && !next.ephemeris.is_valid(sv, t, next.toe) {
                    self.buffered.push(next);
                    break;
                } else {
                    self.buffered.push(next);
                }
            } else {
                break;
            }
        }

        let buffered = self
            .buffered
            .iter()
            .filter(|k| k.ephemeris.is_valid(sv, t, k.toe))
            .min_by_key(|k| k.toe - t)?;

        buffered.to_orbit(t)
    }
}

impl QcContext {
    /// Obtain [QcEphemerisBuffer] from this [QcContext] and navigation using
    /// provided [Frame].
    pub fn ephemeris_buffer<'a>(&'a self, frame: Frame) -> Option<QcEphemerisBuffer<'a>> {
        let brdc = self.brdc_navigation()?;

        Some(QcEphemerisBuffer {
            frame,
            buffered: Vec::with_capacity(8),
            iter: Box::new(brdc.nav_ephemeris_frames_iter().filter_map(|(k, v)| {
                let sv_ts = k.sv.constellation.timescale()?;
                let toe = v.toe(sv_ts)?;
                Some(QcEphemerisData {
                    toe,
                    toc: k.epoch,
                    sv: k.sv,
                    ephemeris: v.clone(),
                })
            })),
        })
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::prelude::{Epoch, QcContext, SV};

    #[test]
    fn ephemeris_buffering() {
        let mut ctx = QcContext::new();

        // load other type of data
        ctx.load_rinex_file("data/MET/V2/abvi0010.15m").unwrap();

        assert!(
            ctx.ephemeris_buffer(ctx.earth_cef).is_none(),
            "non existing ephemeris!"
        );

        // load NAV
        ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
            .unwrap();

        let mut ephemeris = ctx
            .ephemeris_buffer(ctx.earth_cef)
            .expect("ephemeris buffer should exist!");

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

        // while let Some(signal) = signals.next() {
        //     if signal.t == t0 {
        //         if signal.sv == g01 {
        //             match (signal.carrier, signal.measurement) {
        //                 (Carrier::L1, QcMeasuredData::PseudoRange(_)) => {
        //                     if t0_g01_found {
        //                         panic!("Iterator proposed duplicated sample: {}/{}", t0, g01);
        //                     }
        //                     t0_g01_found = true;
        //                 }
        //                 _ => {} // not tested
        //             }
        //         } else if signal.sv == r24 {
        //             match (signal.carrier, signal.measurement) {
        //                 (Carrier::G1(_), QcMeasuredData::PseudoRange(_)) => {
        //                     if t0_r24_found {
        //                         panic!("Iterator proposed duplicated sample: {}/{}", t0, r24);
        //                     }
        //                     t0_r24_found = true;
        //                 }
        //                 _ => {} // not tested
        //             }
        //         } else {
        //             // not tested
        //         }
        //     } else if signal.t == t1 {
        //         if signal.sv == g01 {
        //             match (signal.carrier, signal.measurement) {
        //                 (Carrier::L1, QcMeasuredData::PseudoRange(_)) => {
        //                     if t1_g01_found {
        //                         panic!("Iterator proposed duplicated sample: {}/{}", t1, g01);
        //                     }
        //                     t1_g01_found = true;
        //                 }
        //                 _ => {} // not tested
        //             }
        //         } else if signal.sv == r24 {
        //             match (signal.carrier, signal.measurement) {
        //                 (Carrier::G1(_), QcMeasuredData::PseudoRange(_)) => {
        //                     if t1_r24_found {
        //                         panic!("Iterator proposed duplicated sample: {}/{}", t1, r24);
        //                     }
        //                     t1_r24_found = true;
        //                 }
        //                 _ => {} // not tested
        //             }
        //         } else {
        //             // not tested
        //         }
        //     } else if signal.t == t2 {
        //         if signal.sv == g01 {
        //             match (signal.carrier, signal.measurement) {
        //                 (Carrier::L1, QcMeasuredData::PseudoRange(_)) => {
        //                     if t2_g01_found {
        //                         panic!("Iterator proposed duplicated sample: {}/{}", t2, g01);
        //                     }
        //                     t2_g01_found = true;
        //                 }
        //                 _ => {} // not tested
        //             }
        //         } else if signal.sv == r24 {
        //             match (signal.carrier, signal.measurement) {
        //                 (Carrier::G1(_), QcMeasuredData::PseudoRange(_)) => {
        //                     if t2_r24_found {
        //                         panic!("Iterator proposed duplicated sample: {}/{}", t2, r24);
        //                     }
        //                     t2_r24_found = true;
        //                 }
        //                 _ => {} // not tested
        //             }
        //         } else {
        //             // not tested
        //         }
        //     } else if signal.t == t3 {
        //         if signal.sv == g01 {
        //             match (signal.carrier, signal.measurement) {
        //                 (Carrier::L1, QcMeasuredData::PseudoRange(_)) => {
        //                     if t3_g01_found {
        //                         panic!("Iterator proposed duplicated sample: {}/{}", t3, g01);
        //                     }
        //                     t3_g01_found = true;
        //                 }
        //                 _ => {} // not tested
        //             }
        //         } else if signal.sv == r24 {
        //             match (signal.carrier, signal.measurement) {
        //                 (Carrier::G1(_), QcMeasuredData::PseudoRange(_)) => {
        //                     if t3_r24_found {
        //                         panic!("Iterator proposed duplicated sample: {}/{}", t3, r24);
        //                     }
        //                     t3_r24_found = true;
        //                 }
        //                 _ => {} // not tested
        //             }
        //         } else {
        //             // not tested
        //         }
        //     } else {
        //         panic!("Iterator proposed incorrect {} epoch!", signal.t);
        //     }
        // }

        // assert!(t0_g01_found, "T0/G01 data is missing!");
    }
}
