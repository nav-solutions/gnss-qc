//! Synthetic data generators for unit tests
use std::str::FromStr;

use gnss_rs::prelude::{Constellation, SV};
use hifitime::{Duration, Epoch, TimeScale};

use rinex::{
    navigation::{Ephemeris, OrbitItem},
    prelude::{
        obs::{EpochFlag, SignalObservation},
        Observable,
    },
};

use crate::io::source::{QcEphemeris, QcInputItem};

/// GPS ephemeris, published at ToC = ToE = `toc`.
pub fn gps_ephemeris(sv: SV, toc: Epoch) -> QcEphemeris {
    let (week, nanos) = toc.to_time_scale(TimeScale::GPST).to_time_of_week();
    let ephemeris = Ephemeris::default()
        .with_week(week)
        .with_orbit("toe", OrbitItem::from(nanos as f64 * 1.0E-9));

    assert_eq!(ephemeris.toe(sv), Some(toc), "test ephemeris toe");

    QcEphemeris {
        sv,
        toc,
        ephemeris: Box::new(ephemeris),
    }
}

/// Ephemeris input item
pub fn ephemeris_item(sv: &str, toc: Epoch) -> QcInputItem {
    let sv = SV::from_str(sv).unwrap();
    let eph = gps_ephemeris(sv, toc);
    QcInputItem::ephemeris(toc, sv, *eph.ephemeris)
}

/// One pseudo range observation item
pub fn pseudo_range_item(sv: &str, t: Epoch, value: f64) -> QcInputItem {
    let sv = SV::from_str(sv).unwrap();
    assert_eq!(sv.constellation, Constellation::GPS);
    let observable = Observable::from_str("C1C").unwrap();
    QcInputItem::signal(
        t,
        EpochFlag::Ok,
        SignalObservation::new(sv, observable, value),
    )
}

/// Observations: `svs` sampled every `dt` from `start` (included) to `end` (excluded)
pub fn observations(svs: &[&str], start: Epoch, end: Epoch, dt: Duration) -> Vec<QcInputItem> {
    let mut items = Vec::new();
    let mut t = start;
    while t < end {
        for sv in svs {
            items.push(pseudo_range_item(sv, t, 20.0E6));
        }
        t += dt;
    }
    items
}

pub fn epoch(s: &str) -> Epoch {
    Epoch::from_str(s).unwrap()
}
