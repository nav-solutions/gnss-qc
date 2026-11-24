//! Ionospheric Pierce Points: ESBC00DNK, 2020-06-25 00:00-01:00, NAV + SP3.
use std::collections::HashMap;

use crate::{geodesy::ecef_to_geodetic, prelude::*, tests::miso::load_products};

const ESBC_OBS: &str = "CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz";
const ESBC_NAV: &str = "NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz";
const GRG_SP3: &str = "SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz";

/// ESBC00DNK marker (RINEX header)
const ESBC_ECEF_M: (f64, f64, f64) = (3582105.291, 532589.7313, 5232754.8054);

fn epoch(s: &str) -> Epoch {
    use std::str::FromStr;
    Epoch::from_str(s).unwrap()
}

/// Runs the IPP processor with both orbital products (same engine, same stream)
fn run() -> Vec<(Epoch, QcIppOutput)> {
    let products = load_products(&QcInputConfig::default(), &[ESBC_OBS, ESBC_NAV, GRG_SP3]);

    let precise = QcIppProcessor::new(QcIppConfig {
        orbits: QcOrbitPreference::PreciseOnly,
        ..Default::default()
    });

    let broadcast = QcIppProcessor::new(QcIppConfig {
        orbits: QcOrbitPreference::BroadcastOnly,
        ..Default::default()
    });

    let t0 = epoch("2020-06-25T00:00:00 GPST");

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_time_window(Some(t0), Some(t0 + Duration::from_hours(1.0)))
        .with_processor(precise)
        .with_processor(broadcast)
        .build()
        .unwrap();

    engine
        .filter_map(|item| {
            let item = item.unwrap();
            match item.payload {
                QcOutput::IPP(ipp) => Some((item.epoch, ipp)),
                _ => None,
            }
        })
        .collect()
}

#[test]
fn ipp_geometry_and_products() {
    let ipps = run();

    let (lat, long, _) = ecef_to_geodetic(ESBC_ECEF_M);
    let (lat, long) = (lat.to_degrees(), long.to_degrees());

    let precise = ipps
        .iter()
        .filter(|(_, ipp)| ipp.orbit == QcOrbitKind::Precise)
        .map(|(t, ipp)| ((*t, ipp.sv), ipp))
        .collect::<HashMap<_, _>>();

    let broadcast = ipps
        .iter()
        .filter(|(_, ipp)| ipp.orbit == QcOrbitKind::Broadcast)
        .collect::<Vec<_>>();

    // precise products cover GPS, Galileo and Glonass (Glonass has no Keplerian orbit),
    // broadcast ephemerides cover GPS, Galileo and BeiDou
    assert!(precise.len() > 2500, "{}", precise.len());
    assert!(broadcast.len() > 2500, "{}", broadcast.len());
    assert!(precise
        .keys()
        .any(|(_, sv)| sv.constellation == Constellation::Glonass));
    assert!(broadcast
        .iter()
        .any(|(_, ipp)| ipp.sv.constellation == Constellation::BeiDou));

    let mut compared = 0;

    for (t, ipp) in ipps.iter() {
        assert_eq!(ipp.receiver.to_string(), "receiver:SEPT POLARX5 #3047937");

        // elevation mask, default shell
        assert!(ipp.elevation_deg >= 10.0 && ipp.elevation_deg <= 90.0);
        assert!(ipp.azimuth_deg >= 0.0 && ipp.azimuth_deg < 360.0);
        assert_eq!(ipp.altitude_km, 450.0);

        // single layer model at 450 km, 10° elevation mask: within ~15° of the station
        assert!((ipp.latitude_deg - lat).abs() < 16.0, "{:?}", ipp);
        assert!(
            ((ipp.longitude_deg - long) * lat.to_radians().cos()).abs() < 16.0,
            "{:?}",
            ipp
        );

        // obliquity factor: 1 at zenith, ~3 at 10° elevation
        assert!(
            ipp.obliquity_factor >= 1.0 && ipp.obliquity_factor < 3.0,
            "{:?}",
            ipp
        );

        // broadcast versus precise orbits: same pierce points (~meters)
        if ipp.orbit == QcOrbitKind::Broadcast {
            if let Some(precise) = precise.get(&(*t, ipp.sv)) {
                assert!((precise.elevation_deg - ipp.elevation_deg).abs() < 1.0E-3);
                assert!((precise.latitude_deg - ipp.latitude_deg).abs() < 1.0E-3);
                assert!((precise.longitude_deg - ipp.longitude_deg).abs() < 1.0E-3);
                compared += 1;
            }
        }
    }

    assert!(compared > 1500, "{}", compared);
}

#[test]
fn ipp_time_series() {
    let ipps = run();

    // per satellite time series (precise orbits)
    let mut series = HashMap::<SV, Vec<(Epoch, &QcIppOutput)>>::new();
    for (t, ipp) in ipps
        .iter()
        .filter(|(_, ipp)| ipp.orbit == QcOrbitKind::Precise)
    {
        series.entry(ipp.sv).or_default().push((*t, ipp));
    }

    assert!(series.len() > 15, "{} satellites", series.len());

    for (sv, points) in series.iter() {
        for pair in points.windows(2) {
            let (t0, a) = pair[0];
            let (t1, b) = pair[1];

            if t1 - t0 > Duration::from_seconds(30.0) {
                continue; // data gap
            }

            // pierce points move continuously (faster at low elevation)
            assert!(
                (b.latitude_deg - a.latitude_deg).abs() < 0.3,
                "{} {}",
                sv,
                t1
            );
            assert!(
                (b.longitude_deg - a.longitude_deg).abs() < 0.5,
                "{} {}",
                sv,
                t1
            );

            // obliquity factor follows the elevation
            if b.elevation_deg > a.elevation_deg + 1.0E-3 {
                assert!(b.obliquity_factor < a.obliquity_factor);
            }
        }
    }
}

/// TEC maps (IONEX) covering the session: the vertical TEC at each pierce point is
/// interpolated from the maps. Synthetic worldwide maps (hourly, 2.5° x 5°):
/// TEC = 10 + 0.1 latitude + hour, bilinear (and linear in time) interpolation is exact.
#[test]
fn ipp_vertical_tec() {
    let t0 = epoch("2020-06-25T00:00:00 GPST");

    let tec = |hour: f64, lat: f64| 10.0 + 0.1 * lat + hour;

    let mut items = Vec::new();

    for hour in 0..3 {
        let mut lat = -87.5;
        while lat <= 87.5 {
            let mut lon = -180.0;
            while lon <= 180.0 {
                items.push(QcInputItem::tec(
                    t0 + Duration::from_hours(hour as f64),
                    QcTecPoint {
                        latitude_deg: lat,
                        longitude_deg: lon,
                        altitude_km: 450.0,
                        tecu: tec(hour as f64, lat),
                        rms_tecu: None,
                    },
                ));
                lon += 5.0;
            }
            lat += 2.5;
        }
    }

    let maps = QcIterSource::new(
        QcSourceDescriptor::new(QcIndexing::Agency("TEST".to_string())).with_tec_maps(),
        items.into_iter(),
    );

    let products = load_products(&QcInputConfig::default(), &[ESBC_OBS, ESBC_NAV]);

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_source(maps)
        .with_time_window(Some(t0), Some(t0 + Duration::from_hours(1.0)))
        .with_processor(QcIppProcessor::new(QcIppConfig::default()))
        .build()
        .unwrap();

    let mut n = 0;

    for item in engine {
        let item = item.unwrap();
        if let QcOutput::IPP(ipp) = item.payload {
            let hour = (item.epoch - t0).to_seconds() / 3600.0;
            let expected = tec(hour, ipp.latitude_deg);

            let vertical = ipp.vertical_tecu.unwrap();
            assert!(
                (vertical - expected).abs() < 1.0E-6,
                "{} {}",
                vertical,
                expected
            );

            let slant = ipp.slant_tecu().unwrap();
            assert!((slant - vertical * ipp.obliquity_factor).abs() < 1.0E-9);

            // L1: 0.162 m / TECu
            let delay = ipp.ionospheric_delay_m(1575.42E6).unwrap();
            assert!((delay / slant - 0.1624).abs() < 1.0E-3);

            n += 1;
        }
    }

    assert!(n > 1000, "{}", n);
}
