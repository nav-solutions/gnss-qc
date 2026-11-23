//! Residual analysis: observation residuals (receiver A - receiver B)
//! and orbit residuals (product A - product B).
use std::collections::{BTreeMap, HashMap};

use crate::{
    prelude::*,
    tests::{
        miso::{data_path, load_products, load_rinex},
        toolkit::pseudo_range_item,
    },
};

const ESBC_OBS: &str = "CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz";
const MOJN_OBS: &str = "CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz";
const SDR_OBS: &str = "OBS/V3/gps.23O.gz";
const POLARX5_OBS: &str = "OBS/V3/OB713520.23O.gz";
const ESBC_NAV: &str = "NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz";
const MOJN_NAV: &str = "NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz";
#[cfg(feature = "sp3")]
const GRG_SP3: &str = "SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz";
#[cfg(feature = "sp3")]
const IAC_SP3: &str = "SP3/D/Sta21114.sp3.gz";

const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;

fn epoch(s: &str) -> Epoch {
    use std::str::FromStr;
    Epoch::from_str(s).unwrap()
}

fn receiver(name: &str) -> QcIndexing {
    QcIndexing::Receiver(name.to_string())
}

fn run(engine: QcEngine<QcOutput>) -> Vec<(Epoch, QcOutput)> {
    engine
        .map(|item| {
            let item = item.unwrap();
            (item.epoch, item.payload)
        })
        .collect()
}

fn observation_residuals(results: &[(Epoch, QcOutput)]) -> Vec<(Epoch, &QcObservationResidual)> {
    results
        .iter()
        .filter_map(|(t, output)| match output {
            QcOutput::ObservationResidual(residual) => Some((*t, residual)),
            _ => None,
        })
        .collect()
}

fn orbit_residuals(results: &[(Epoch, QcOutput)]) -> Vec<(Epoch, &QcOrbitResidual)> {
    results
        .iter()
        .filter_map(|(t, output)| match output {
            QcOutput::OrbitResidual(residual) => Some((*t, residual)),
            _ => None,
        })
        .collect()
}

/// Streamed observation residuals are identical to `Rinex::observations_substract`
/// (ESBC00DNK - MOJN00DNK, 30', every observable).
#[test]
fn observation_residuals_match_rinex() {
    let end = epoch("2020-06-25T00:30:00 GPST");

    let truncate = |file: &str| {
        let mut rinex = load_rinex(file);
        rinex
            .record
            .as_mut_obs()
            .unwrap()
            .retain(|k, _| k.epoch < end);
        rinex
    };

    let (esbc, mojn) = (truncate(ESBC_OBS), truncate(MOJN_OBS));

    // reference: file based differentiation
    let reference = esbc.observations_substract(&mojn).unwrap();

    let mut expected = reference
        .record
        .as_obs()
        .unwrap()
        .iter()
        .flat_map(|(k, obs)| {
            obs.signals
                .iter()
                .map(move |sig| (k.epoch, sig.sv, sig.observable.clone(), sig.value))
        })
        .collect::<Vec<_>>();

    // streamed: MISO engine
    let mut products = QcInputProducts::default();
    let cfg = QcInputConfig::default();
    products
        .load_rinex(&cfg, data_path(ESBC_OBS), esbc)
        .unwrap();
    products
        .load_rinex(&cfg, data_path(MOJN_OBS), mojn)
        .unwrap();

    let processor = QcObservationResidualProcessor::new(
        QcObservationResidualConfig::new(
            receiver("SEPT POLARX5 #3047937"),
            receiver("SEPT POLARX5 #3022663"),
        )
        .with_all_observables(),
    );

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(processor)
        .build()
        .unwrap());

    let mut streamed = observation_residuals(&results)
        .into_iter()
        .map(|(t, r)| {
            assert_eq!(r.rhs_epoch, t);
            (t, r.sv, r.observable.clone(), r.residual)
        })
        .collect::<Vec<_>>();

    expected.sort_by(|a, b| a.partial_cmp(b).unwrap());
    streamed.sort_by(|a, b| a.partial_cmp(b).unwrap());

    assert!(expected.len() > 30_000, "{}", expected.len());
    assert_eq!(streamed.len(), expected.len());
    assert_eq!(streamed, expected);
}

/// Observation residuals between a software receiver (GNSS-SDR) and a geodetic receiver
/// (PolaRx5), 2023-12-18, ~2 hours: the pseudo range residuals describe the relative clock
/// state, the phase residuals reveal a receiver level phase divergence.
#[test]
fn observation_residuals_clock_analysis() {
    let products = load_products(&QcInputConfig::default(), &[SDR_OBS, POLARX5_OBS]);

    let sdr = receiver("Software Receiver #GNSS-SDR");
    let polarx5 = receiver("SEPT POLARX5TR #3088149");

    let processor =
        QcObservationResidualProcessor::new(QcObservationResidualConfig::new(polarx5, sdr));

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(processor)
        .build()
        .unwrap());

    let residuals = observation_residuals(&results);

    // only pseudo range and phase (default), only the common observables (C1C, L1C)
    assert!(residuals.len() > 1500, "{}", residuals.len());
    for (_, r) in residuals.iter() {
        let code = r.observable.to_string();
        assert!(code == "C1C" || code == "L1C", "{}", code);
    }

    // pseudo range residuals: common to all satellites (same geometry),
    // ~ -9.433 ms of clock difference, stable over 2 hours
    let mut code_per_epoch = BTreeMap::<Epoch, Vec<f64>>::new();
    let mut series = HashMap::<(SV, String), Vec<(f64, f64)>>::new();
    let t0 = residuals[0].0;

    for (t, r) in residuals.iter() {
        let lambda = SPEED_OF_LIGHT_M_S / 1575.42E6;
        let value_m = if r.observable.is_phase_range_observable() {
            r.residual * lambda
        } else {
            code_per_epoch.entry(*t).or_default().push(r.residual);
            r.residual
        };

        series
            .entry((r.sv, r.observable.to_string()))
            .or_default()
            .push(((*t - t0).to_seconds(), value_m));
    }

    for (t, values) in code_per_epoch.iter() {
        let min = values.iter().copied().fold(f64::MAX, f64::min);
        let max = values.iter().copied().fold(f64::MIN, f64::max);
        assert!(
            max - min < 40.0,
            "{}: code residuals spread {} m",
            t,
            max - min
        );

        let clock_ms = values[0] / SPEED_OF_LIGHT_M_S * 1.0E3;
        assert!((clock_ms + 9.433).abs() < 0.001, "{}: {} ms", t, clock_ms);
    }

    // linear trends (m/s) of the long arcs
    let slope = |points: &[(f64, f64)]| {
        let n = points.len() as f64;
        let mx = points.iter().map(|p| p.0).sum::<f64>() / n;
        let my = points.iter().map(|p| p.1).sum::<f64>() / n;
        points.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum::<f64>()
            / points.iter().map(|p| (p.0 - mx).powi(2)).sum::<f64>()
    };

    let long_arcs = series
        .iter()
        .filter(|(_, points)| points.len() > 200)
        .collect::<Vec<_>>();

    assert!(long_arcs.len() >= 8);

    let code_slopes = long_arcs
        .iter()
        .filter(|((_, code), _)| code == "C1C")
        .map(|(_, points)| slope(points))
        .collect::<Vec<_>>();

    let phase_slopes = long_arcs
        .iter()
        .filter(|((_, code), _)| code == "L1C")
        .map(|(_, points)| slope(points))
        .collect::<Vec<_>>();

    // syntonized clocks: no code drift
    for s in code_slopes.iter() {
        assert!(s.abs() < 0.01, "code drift {} m/s", s);
    }

    // phase drift diverging from the code on every long arc,
    // common to most satellites (receiver level, ~ -0.48 m/s)
    let mut sorted = phase_slopes.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = sorted[sorted.len() / 2];

    assert!((median + 0.482).abs() < 0.01, "{:?}", phase_slopes);

    let common = phase_slopes
        .iter()
        .filter(|s| (*s - median).abs() < 0.01)
        .count();

    assert!(common >= 3, "{:?}", phase_slopes);

    for s in phase_slopes.iter() {
        assert!(*s < -0.1, "{:?}", phase_slopes);
    }
}

/// Receivers sampling at slightly different instants: residuals are formed with the
/// closest epoch of B within the tolerance, whatever the order of arrival.
#[test]
fn observation_residuals_misaligned_receivers() {
    let t0 = epoch("2020-06-25T00:00:00 GPST");
    let dt = Duration::from_seconds(30.0);
    let offset = Duration::from_milliseconds(120.0);

    // A: 0, 30, 60.. B: 0.12, 30.12.. (B samples after A)
    let a = (0..10)
        .map(|i| pseudo_range_item("G01", t0 + dt * i, 1000.0 + i as f64))
        .collect::<Vec<_>>();
    let b = (0..10)
        .map(|i| pseudo_range_item("G01", t0 + dt * i + offset, 900.0))
        .collect::<Vec<_>>();

    let source = |name: &str, items: Vec<QcInputItem>| {
        QcIterSource::new(
            QcSourceDescriptor::new(receiver(name)).with_signals(),
            items.into_iter(),
        )
    };

    let results = run(QcEngine::builder()
        .with_source(source("A", a))
        .with_source(source("B", b))
        .with_processor(QcObservationResidualProcessor::new(
            QcObservationResidualConfig::new(receiver("A"), receiver("B")),
        ))
        .build()
        .unwrap());

    let residuals = observation_residuals(&results);

    // the first epoch of A: sampling interval unknown, exact match required
    assert_eq!(residuals.len(), 9);

    for (i, (t, r)) in residuals.iter().enumerate() {
        let i = i + 1;
        assert_eq!(*t, t0 + dt * i as i64);
        assert_eq!(r.rhs_epoch, t0 + dt * i as i64 + offset);
        assert_eq!(r.residual, 100.0 + i as f64);
    }
}

/// Precise products of two agencies (GRG - IAC), 2020-06-25: centimeter level.
#[test]
#[cfg(feature = "sp3")]
fn orbit_residuals_precise() {
    let products = load_products(&QcInputConfig::default(), &[GRG_SP3, IAC_SP3]);

    let grg = QcIndexing::Agency("GRGS".to_string());
    let iac = QcIndexing::Agency("IAC".to_string());

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(QcOrbitResidualProcessor::new(
            QcOrbitResidualConfig::precise(grg.clone(), iac.clone()),
        ))
        .build()
        .unwrap());

    let residuals = orbit_residuals(&results);

    // 96 epochs (15'), satellites described by both agencies
    assert!(residuals.len() > 7000, "{}", residuals.len());

    let mut stats = HashMap::<Constellation, (usize, f64)>::new();

    for (_, r) in residuals.iter() {
        assert_eq!(r.lhs_source, grg);
        assert_eq!(r.rhs_source, iac);
        assert_eq!(r.lhs_kind, QcOrbitKind::Precise);
        assert!(r.lhs_toc.is_none());

        // RAC decomposition preserves the norm
        let rac = (r.radial_m.powi(2) + r.along_track_m.powi(2) + r.cross_track_m.powi(2)).sqrt();
        assert!((rac - r.norm_m()).abs() < 1.0E-6);

        assert!(r.norm_m() < 0.25, "{} {}", r.sv, r.norm_m());

        // clock residuals: nanoseconds (clock datum of each agency)
        assert!(
            r.clock_s.unwrap().abs() < 10.0E-9,
            "{} {:?}",
            r.sv,
            r.clock_s
        );

        let s = stats.entry(r.sv.constellation).or_default();
        s.0 += 1;
        s.1 += r.norm_m().powi(2);
    }

    for (constellation, (n, sum)) in stats.iter() {
        let rms = (sum / *n as f64).sqrt();
        assert!(rms < 0.06, "{}: {} m RMS", constellation, rms);
    }

    assert!(stats.contains_key(&Constellation::GPS));
    assert!(stats.contains_key(&Constellation::Galileo));
    assert!(stats.contains_key(&Constellation::Glonass));
}

/// Broadcast ephemerides - precise products (ESBC00DNK NAV - GRG), every 15',
/// recent ephemerides only: meter level.
#[test]
#[cfg(feature = "sp3")]
fn orbit_residuals_broadcast_precise() {
    let products = load_products(&QcInputConfig::default(), &[ESBC_NAV, GRG_SP3]);

    let cfg = QcOrbitResidualConfig::broadcast_precise()
        .with_sampling(Duration::from_seconds(900.0))
        .with_max_broadcast_age(Duration::from_hours(1.0));

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(QcOrbitResidualProcessor::new(cfg))
        .build()
        .unwrap());

    let residuals = orbit_residuals(&results);

    let mut stats = HashMap::<Constellation, (usize, f64, f64)>::new();

    for (t, r) in residuals.iter() {
        assert_eq!(r.lhs_kind, QcOrbitKind::Broadcast);
        assert_eq!(r.rhs_kind, QcOrbitKind::Precise);
        assert!((*t - r.lhs_toc.unwrap()).abs() <= Duration::from_hours(1.0));

        // eccentric Galileo satellites (launch anomaly, testing status in 2020)
        if r.sv.constellation == Constellation::Galileo && (r.sv.prn == 14 || r.sv.prn == 18) {
            continue;
        }

        let s = stats.entry(r.sv.constellation).or_default();
        s.0 += 1;
        s.1 += r.norm_m().powi(2);
        s.2 = s.2.max(r.norm_m());
    }

    for constellation in [Constellation::GPS, Constellation::Galileo] {
        let (n, sum, max) = stats[&constellation];
        let rms = (sum / n as f64).sqrt();
        assert!(n > 1000, "{}: {}", constellation, n);
        assert!(rms < 2.5, "{}: {} m RMS", constellation, rms);
        assert!(max < 6.0, "{}: {} m max", constellation, max);
    }
}

/// Broadcast ephemerides decoded by two stations (ESBC00DNK - MOJN00DNK):
/// identical ephemerides give identical orbits. Galileo clocks reveal the mixing of
/// I/NAV and F/NAV messages (same time of clock, different clock references).
#[test]
fn orbit_residuals_broadcast() {
    let products = load_products(&QcInputConfig::default(), &[ESBC_NAV, MOJN_NAV]);

    let esbc = QcIndexing::Custom("ESBC00DNK_R_20201770000_01D_MN".to_string());
    let mojn = QcIndexing::Custom("MOJN00DNK_R_20201770000_01D_MN".to_string());

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(QcOrbitResidualProcessor::new(
            QcOrbitResidualConfig::broadcast(esbc, mojn)
                .with_sampling(Duration::from_seconds(900.0)),
        ))
        .build()
        .unwrap());

    let residuals = orbit_residuals(&results);
    assert!(residuals.len() > 1000, "{}", residuals.len());

    let (mut same, mut galileo_clock_mismatches) = (0, 0);

    for (_, r) in residuals.iter() {
        if r.lhs_toc == r.rhs_toc {
            same += 1;

            // same ephemeris, same orbit
            assert!(r.norm_m() < 1.0E-6, "{} {}", r.sv, r.norm_m());

            let clock = r.clock_s.unwrap();

            if r.sv.constellation == Constellation::Galileo {
                if clock.abs() > 1.0E-10 {
                    galileo_clock_mismatches += 1;
                }
            } else {
                assert!(clock.abs() < 1.0E-15, "{} {}", r.sv, clock);
            }
        }
    }

    assert!(same > 1000);
    assert!(galileo_clock_mismatches > 0);
}
