//! Keplerian solver cross validation: broadcast (NAV) versus precise (SP3) orbits
use std::collections::HashMap;

use super::*;
use crate::navigation::QcKepler;

use sp3::prelude::SP3;

#[test]
fn kepler_versus_sp3() {
    let nav = load_rinex(ESBC_NAV);
    let sp3 = SP3::from_gzip_file(data_path(GRG_SP3)).unwrap();

    let ephemerides = nav
        .record
        .as_nav()
        .unwrap()
        .iter()
        .filter_map(|(k, frame)| Some((k.sv, k.epoch, frame.as_ephemeris()?.clone())))
        .collect::<Vec<_>>();

    // worst error, per constellation (m)
    let mut worst = HashMap::<Constellation, f64>::new();
    let mut compared = 0;

    for (key, entry) in sp3.data.iter() {
        let t = key.epoch;
        let sv = key.sv;

        // E14 and E18: eccentric orbits (launch anomaly), "testing" status in 2020,
        // normally discarded by their health flag.
        if sv.constellation == Constellation::Galileo && (sv.prn == 14 || sv.prn == 18) {
            continue;
        }

        // one sample every 2 hours
        if (t - epoch("2020-06-25T00:00:00 GPST")).to_seconds() % 7200.0 != 0.0 {
            continue;
        }

        // closest valid ephemeris. Single station NAV files only contain the
        // ephemerides of the satellites in sight: satellites out of sight would only
        // have distant (extrapolated) ephemerides. We restrict to |t - toc| < 1h,
        // which is the case of any satellite in sight.
        let eph = ephemerides
            .iter()
            .filter(|(e_sv, _, eph)| *e_sv == sv && eph.is_valid(sv, t))
            .filter(|(_, toc, _)| (*toc - t).abs() < Duration::from_hours(1.0))
            .min_by_key(|(_, toc, _)| (*toc - t).abs());

        let (_, toc, eph) = match eph {
            Some(eph) => eph,
            None => continue,
        };

        let kepler = match QcKepler::from_ephemeris(sv, *toc, eph) {
            Some(kepler) => kepler,
            None => continue,
        };

        let state = kepler.state(t);
        let (x, y, z) = state.position_m;
        let (xs, ys, zs) = entry.position_km;

        let err =
            ((x - xs * 1.0E3).powi(2) + (y - ys * 1.0E3).powi(2) + (z - zs * 1.0E3).powi(2)).sqrt();

        let w = worst.entry(sv.constellation).or_default();
        *w = w.max(err);
        compared += 1;
    }

    println!("compared: {} worst errors: {:?}", compared, worst);

    assert!(compared > 250);

    // broadcast orbits accuracy (~1 m) + antenna phase center / center of mass offset
    for (constellation, err) in worst.iter() {
        assert!(*err < 5.0, "{}: kepler error {} m", constellation, err);
    }
}
