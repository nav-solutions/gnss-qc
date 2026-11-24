//! Clock RINEX and Meteo RINEX inputs: sources, synchronization, processors.
use std::collections::{HashMap, VecDeque};

use rinex::{clock::ClockProfileType, prelude::Observable};

use crate::{
    io::rinex::{QcRinexClockSource, QcRinexMeteoSource},
    prelude::*,
    sync::interpolate_linear,
    tests::miso::{data_path, load_products, load_rinex},
};

const GRG_CLK: &str = "CLK/V3/GRG0MGXFIN_20201770000_01D_30S_CLK.CLK.gz";
const COD_CLK: &str = "CLK/V3/COD0MGXFIN_20241280000_01D_30S_CLK.CLK.gz";
const POTS_MET: &str = "MET/V3/POTS00DEU_R_20232540000_01D_05M_MM.rnx.gz";
const ABVI_MET: &str = "MET/V2/abvi0010.15m";

#[cfg(feature = "sp3")]
const GRG_SP3: &str = "SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz";

const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;

fn run(engine: QcEngine<QcOutput>) -> Vec<(Epoch, QcOutput)> {
    engine
        .map(|item| {
            let item = item.unwrap();
            (item.epoch, item.payload)
        })
        .collect()
}

fn precise_clocks(results: &[(Epoch, QcOutput)]) -> Vec<(Epoch, &QcPreciseClockOutput)> {
    results
        .iter()
        .filter_map(|(t, output)| match output {
            QcOutput::PreciseClock(clock) => Some((*t, clock)),
            _ => None,
        })
        .collect()
}

fn meteo(results: &[(Epoch, QcOutput)]) -> Vec<(Epoch, &QcMeteoOutput)> {
    results
        .iter()
        .filter_map(|(t, output)| match output {
            QcOutput::Meteo(sample) => Some((*t, sample)),
            _ => None,
        })
        .collect()
}

/// Clock RINEX: indexed by analysis center, every state delivered in chronological order
#[test]
fn clock_rinex_source() {
    let products = load_products(&QcInputConfig::default(), &[GRG_CLK, COD_CLK]);

    let mut agencies = products
        .clk_rinex_iter()
        .map(|(k, _)| k.clone())
        .collect::<Vec<_>>();
    agencies.sort_by_key(|k| k.to_string());

    // COD MGEX products: analysis center code COM
    assert_eq!(
        agencies,
        vec![
            QcIndexing::Agency("COM".to_string()),
            QcIndexing::Agency("GRG".to_string())
        ]
    );

    let rinex = load_rinex(GRG_CLK);
    let expected = rinex.precise_sv_clock().count();
    assert_eq!(rinex.precise_station_clock().count(), 0);

    let mut source = QcRinexClockSource::new(QcIndexing::Agency("GRG".to_string()), rinex).unwrap();

    assert!(source.descriptor().precise_clocks);
    assert!(source.descriptor().provides_context());
    assert!(!source.descriptor().signals);

    let (mut count, mut last) = (0, None);

    while let QcPoll::Ready(item) = source.poll().unwrap() {
        assert!(last.is_none_or(|t| item.epoch >= t));
        last = Some(item.epoch);

        match item.payload {
            QcPayload::PreciseClock(clock) => {
                assert_eq!(clock.epoch, item.epoch);
                assert_eq!(clock.profile, ClockProfileType::AS);
                assert!(clock.clock.as_satellite().is_some());
                assert!(clock.offset_s.abs() < 1.0E-2, "{:e}", clock.offset_s);
                count += 1;
            }
            _ => panic!("unexpected payload"),
        }
    }

    assert_eq!(count, expected);
    assert!(count > 200_000, "{}", count);

    // not a clock RINEX
    assert!(
        QcRinexClockSource::new(QcIndexing::Custom("POTS".to_string()), load_rinex(POTS_MET))
            .is_err()
    );
}

/// Precise clocks in a context only session: every state of the file is streamed once,
/// stations and satellites, decimation
#[test]
fn precise_clock_processor() {
    let rinex = load_rinex(COD_CLK);

    let satellites = rinex.precise_sv_clock().count();
    let stations = rinex.precise_station_clock().count();
    assert!(stations > 10_000, "{}", stations);

    let products = load_products(&QcInputConfig::default(), &[COD_CLK]);

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(QcPreciseClockProcessor::default())
        .build()
        .unwrap());

    let clocks = precise_clocks(&results);
    assert_eq!(clocks.len(), satellites + stations);

    let mut last = HashMap::<QcClockId, Epoch>::new();

    for (t, output) in clocks.iter() {
        assert_eq!(*t, output.state.epoch);
        assert_eq!(output.source, QcIndexing::Agency("COM".to_string()));

        // once, in chronological order of each clock
        if let Some(previous) = last.insert(output.state.clock.clone(), *t) {
            assert!(*t > previous);
        }
    }

    let n_stations = last.keys().filter(|c| c.as_station().is_some()).count();
    assert!(n_stations > 100, "{}", n_stations);

    // satellites only, every 15'
    let products = load_products(&QcInputConfig::default(), &[COD_CLK]);

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(QcPreciseClockProcessor::new(
            QcPreciseClockConfig::satellites_only().with_sampling(Duration::from_seconds(900.0)),
        ))
        .build()
        .unwrap());

    let clocks = precise_clocks(&results);
    let mut last = HashMap::<QcClockId, Epoch>::new();

    for (t, output) in clocks.iter() {
        assert!(output.state.clock.as_satellite().is_some());

        if let Some(previous) = last.insert(output.state.clock.clone(), *t) {
            assert!(*t - previous >= Duration::from_seconds(900.0));
        }
    }

    // 96 states per satellite
    let n_satellites = clocks
        .iter()
        .map(|(_, c)| c.state.clock.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();

    assert!(n_satellites > 100, "{}", n_satellites);
    assert!(clocks.len() > 95 * n_satellites, "{}", clocks.len());
}

/// GRG Clock RINEX (30") and GRG precise products (SP3, 15'): same analysis, same clock
/// datum. At the SP3 epochs, both products describe the same clock states.
#[test]
#[cfg(feature = "sp3")]
fn clock_rinex_matches_sp3_clocks() {
    let sp3 = sp3::prelude::SP3::from_gzip_file(crate::tests::miso::data_path(GRG_SP3)).unwrap();

    let sp3_clocks = sp3
        .data
        .iter()
        .filter_map(|(k, entry)| Some(((k.epoch, k.sv), entry.clock_us? * 1.0E-6)))
        .collect::<HashMap<_, _>>();

    let products = load_products(&QcInputConfig::default(), &[GRG_CLK]);

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(QcPreciseClockProcessor::new(
            QcPreciseClockConfig::satellites_only(),
        ))
        .build()
        .unwrap());

    let mut compared = 0;

    for (t, output) in precise_clocks(&results) {
        let sv = output.state.clock.as_satellite().unwrap();

        if let Some(sp3_clock) = sp3_clocks.get(&(t, sv)) {
            // SP3: 1E-12 s resolution
            let residual = output.state.offset_s - sp3_clock;
            assert!(residual.abs() < 1.0E-12, "{} {} {:e}", t, sv, residual);
            compared += 1;
        }
    }

    assert!(compared > 7000, "{}", compared);
}

/// Linear interpolation of 15' clock states (sampling of the SP3 clocks) compared
/// to the 30" states of the Clock RINEX: the interpolation error is significant for
/// GPS and Glonass, which is why Clock RINEX are preferred.
#[test]
fn clock_interpolation_error() {
    let rinex = load_rinex(GRG_CLK);

    let mut series = HashMap::<SV, Vec<(Epoch, f64)>>::new();

    for (t, sv, _, profile) in rinex.precise_sv_clock() {
        series.entry(sv).or_default().push((t, profile.bias));
    }

    let mut rms = HashMap::<Constellation, (usize, f64)>::new();

    for (sv, series) in series.iter() {
        let decimated = series.iter().step_by(30).copied().collect::<VecDeque<_>>();

        for (t, offset_s) in series.iter() {
            if let Some(interpolated) = interpolate_linear(&decimated, *t, |s| *s) {
                let error_m = (interpolated - offset_s) * SPEED_OF_LIGHT_M_S;
                let entry = rms.entry(sv.constellation).or_default();
                entry.0 += 1;
                entry.1 += error_m * error_m;
            }
        }
    }

    let rms = |c: Constellation| {
        let (n, sum) = rms[&c];
        (sum / n as f64).sqrt()
    };

    // passive hydrogen masers (Galileo): millimeter level
    assert!(rms(Constellation::Galileo) < 0.01);

    // GPS and Glonass: centimeter level
    assert!(rms(Constellation::GPS) > 0.02 && rms(Constellation::GPS) < 0.06);
    assert!(rms(Constellation::Glonass) > 0.02 && rms(Constellation::Glonass) < 0.06);
}

/// Meteo RINEX: every observation delivered in chronological order
#[test]
fn meteo_rinex_source() {
    let rinex = load_rinex(POTS_MET);
    let expected = rinex.record.as_meteo().unwrap().len();

    let mut source = QcRinexMeteoSource::new(QcIndexing::Agency("GFZ".to_string()), rinex).unwrap();

    assert!(source.descriptor().meteo);
    assert!(source.descriptor().provides_context());

    let (mut count, mut last) = (0, None);

    while let QcPoll::Ready(item) = source.poll().unwrap() {
        assert!(last.is_none_or(|t| item.epoch >= t));
        last = Some(item.epoch);
        assert!(matches!(item.payload, QcPayload::Meteo(_)));
        count += 1;
    }

    assert_eq!(count, expected);

    // not a meteo RINEX
    assert!(
        QcRinexMeteoSource::new(QcIndexing::Custom("GRG".to_string()), load_rinex(GRG_CLK))
            .is_err()
    );
}

/// Meteo observations in a context only session: every observation of each file is
/// streamed once, with its value
#[test]
fn meteo_processor() {
    let mut expected = Vec::new();

    for file in [POTS_MET, ABVI_MET] {
        let rinex = load_rinex(file);
        for (k, value) in rinex.record.as_meteo().unwrap().iter() {
            expected.push((k.epoch, k.observable.clone(), *value));
        }
    }

    let products = load_products(&QcInputConfig::default(), &[POTS_MET, ABVI_MET]);

    let mut sources = products
        .meteo_rinex_iter()
        .map(|(k, _)| k.to_string())
        .collect::<Vec<_>>();
    sources.sort();
    assert_eq!(sources, vec!["abvi0010", "agency:GFZ"]);

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(QcMeteoProcessor::default())
        .build()
        .unwrap());

    let mut streamed = meteo(&results)
        .into_iter()
        .map(|(t, sample)| (t, sample.observable.clone(), sample.value))
        .collect::<Vec<_>>();

    let key = |a: &(Epoch, Observable, f64)| (a.0, a.1.to_string());
    expected.sort_by_key(key);
    streamed.sort_by_key(key);

    assert_eq!(streamed.len(), expected.len());
    assert_eq!(streamed, expected);

    // pressure only
    let products = load_products(&QcInputConfig::default(), &[POTS_MET]);

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(QcMeteoProcessor::new(
            QcMeteoConfig::default().with_observables(&[Observable::Pressure]),
        ))
        .build()
        .unwrap());

    let pressure = meteo(&results);

    // 1 day, 5' sampling
    assert_eq!(pressure.len(), 288);

    for (_, sample) in pressure {
        assert_eq!(sample.observable, Observable::Pressure);
        assert!(
            sample.value > 950.0 && sample.value < 1050.0,
            "{}",
            sample.value
        );
    }
}

/// Meteo observations are available to the processors at any epoch
/// (latest observation, linear interpolation)
#[test]
fn meteo_context() {
    use std::{cell::RefCell, rc::Rc};

    type Probed = Rc<RefCell<Vec<(Epoch, Option<(Epoch, f64)>, Option<f64>)>>>;

    struct Probe(Probed);

    impl QcProcessor<QcOutput> for Probe {
        fn name(&self) -> &str {
            "probe"
        }

        fn process(&mut self, ctx: &QcEpochContext, _: &mut QcOutputQueue<QcOutput>) {
            let source = QcIndexing::Agency("GFZ".to_string());
            let t = ctx.epoch + Duration::from_seconds(150.0);

            self.0.borrow_mut().push((
                ctx.epoch,
                ctx.meteo.latest(&source, &Observable::Temperature, t),
                ctx.meteo
                    .interpolate_from(&source, &Observable::Temperature, t),
            ));
        }
    }

    let probed = Probed::default();
    let products = load_products(&QcInputConfig::default(), &[POTS_MET]);

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(Probe(probed.clone()))
        .build()
        .unwrap());

    assert!(results.is_empty());

    let temperature = load_rinex(POTS_MET)
        .record
        .as_meteo()
        .unwrap()
        .iter()
        .filter(|(k, _)| k.observable == Observable::Temperature)
        .map(|(k, v)| (k.epoch, *v))
        .collect::<Vec<_>>();

    let probed = probed.borrow();

    // context only session: one epoch per meteo epoch
    assert_eq!(probed.len(), temperature.len());

    for (i, (t, latest, interpolated)) in probed.iter().enumerate() {
        assert_eq!(*t, temperature[i].0);

        // 2'30 later: latest observation is the current one,
        // interpolation is half way to the next one (when known)
        assert_eq!(*latest, Some(temperature[i]));

        match temperature.get(i + 1) {
            Some((_, next)) => {
                let expected = (temperature[i].1 + next) / 2.0;
                assert!((interpolated.unwrap() - expected).abs() < 1.0E-9);
            }
            None => assert!(interpolated.is_none()),
        }
    }
}

/// RMS (ns) of the raw and aligned residuals, per constellation ("station" for stations)
fn clock_residual_stats(results: &[(Epoch, QcOutput)]) -> HashMap<String, (usize, f64, f64, f64)> {
    let mut stats = HashMap::<String, (usize, f64, f64, f64)>::new();

    for (_, output) in results.iter() {
        if let QcOutput::ClockResidual(r) = output {
            let key = match &r.clock {
                QcClockId::Satellite(sv) => format!("{:?}", sv.constellation),
                QcClockId::Station(_) => "station".to_string(),
            };

            let entry = stats.entry(key).or_default();
            entry.0 += 1;
            entry.1 += (r.residual_s * 1.0E9).powi(2);

            if let Some(aligned) = r.aligned_s() {
                entry.2 += (aligned * 1.0E9).powi(2);
                entry.3 = entry.3.max((aligned * 1.0E9).abs());
            }
        }
    }

    stats
        .into_iter()
        .map(|(k, (n, raw, aligned, max))| {
            (
                k,
                (n, (raw / n as f64).sqrt(), (aligned / n as f64).sqrt(), max),
            )
        })
        .collect()
}

/// Two analysis centers: GRG (Clock RINEX) - IAC (SP3 clock states), 2020-06-25, every 15'.
/// Raw residuals are dominated by the clock datum of each center (ns); once aligned
/// (one datum per constellation and per epoch), GPS and Galileo agree at the 0.1 ns level.
/// Glonass clocks absorb satellite specific biases (inter frequency biases): ns level.
#[test]
#[cfg(feature = "sp3")]
fn clock_residuals_between_agencies() {
    let products = load_products(
        &QcInputConfig::default(),
        &[GRG_CLK, "SP3/D/Sta21114.sp3.gz"],
    );

    let cfg = QcClockResidualConfig::new(
        QcClockProduct::Clock(Some(QcIndexing::Agency("GRG".to_string()))),
        QcClockProduct::PreciseState(Some(QcIndexing::Agency("IAC".to_string()))),
    )
    .with_sampling(Duration::from_seconds(900.0));

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(QcClockResidualProcessor::new(cfg))
        .build()
        .unwrap());

    for (_, output) in results.iter() {
        if let QcOutput::ClockResidual(r) = output {
            assert_eq!(r.lhs_source, QcIndexing::Agency("GRG".to_string()));
            assert_eq!(r.rhs_source, QcIndexing::Agency("IAC".to_string()));
            assert_eq!(r.lhs_kind, QcClockKind::PreciseClock);
            assert_eq!(r.rhs_kind, QcClockKind::PreciseState);
            assert!(r.datum_s.is_some());
        }
    }

    let stats = clock_residual_stats(&results);

    // 96 epochs
    let (n, raw, aligned, max) = stats["GPS"];
    assert!(n > 2800, "{}", n);
    assert!(raw > 1.0, "{}", raw);
    assert!(aligned < 0.5, "{}", aligned);
    assert!(max < 1.5, "{}", max);

    let (_, raw, aligned, _) = stats["Galileo"];
    assert!(raw > 1.0 && aligned < 0.4, "{} {}", raw, aligned);

    let (_, _, aligned, _) = stats["Glonass"];
    assert!(aligned > 1.0 && aligned < 5.0, "{}", aligned);

    // no alignment
    let products = load_products(
        &QcInputConfig::default(),
        &[GRG_CLK, "SP3/D/Sta21114.sp3.gz"],
    );

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(QcClockResidualProcessor::new(
            QcClockResidualConfig::new(
                QcClockProduct::Clock(None),
                QcClockProduct::PreciseState(None),
            )
            .with_sampling(Duration::from_seconds(900.0))
            .with_alignment(QcClockAlignment::None),
        ))
        .build()
        .unwrap());

    for (_, output) in results.iter() {
        if let QcOutput::ClockResidual(r) = output {
            assert!(r.datum_s.is_none());
        }
    }
}

/// Clock RINEX and SP3 of the same analysis center: identical clocks at the SP3 epochs;
/// in between, the residuals are the interpolation error of the 15' SP3 clock states
#[test]
#[cfg(feature = "sp3")]
fn clock_residuals_same_agency() {
    for sampling in [Some(Duration::from_seconds(900.0)), None] {
        let products = load_products(&QcInputConfig::default(), &[GRG_CLK, GRG_SP3]);

        let cfg = QcClockResidualConfig {
            sampling,
            ..Default::default()
        };

        let results = run(QcEngine::builder()
            .with_products(products)
            .with_processor(QcClockResidualProcessor::new(cfg))
            .build()
            .unwrap());

        let stats = clock_residual_stats(&results);

        for constellation in ["GPS", "Galileo", "Glonass"] {
            let (n, raw, _, _) = stats[constellation];
            assert!(n > 2000);

            match sampling {
                Some(_) => assert!(raw < 1.0E-3, "{}: {} ns", constellation, raw),
                None => assert!(raw < 0.3, "{}: {} ns", constellation, raw),
            }
        }

        if sampling.is_none() {
            // 30": interpolation error of the 15' SP3 clocks (GPS ~4 cm)
            let (_, raw, _, _) = stats["GPS"];
            assert!(raw > 0.05, "{}", raw);
        }
    }
}

/// Two Clock RINEX (two products indexed under distinct aliases): satellite and
/// station clocks; a product compared to itself gives null residuals
#[test]
fn clock_residuals_two_clock_rinex() {
    let mut products = QcInputProducts::default();

    for alias in ["A", "B"] {
        let cfg = QcInputConfig {
            clk_rinex_indexing: Some(QcIndexingConfig::Custom(alias.to_string())),
            ..Default::default()
        };
        products
            .load_rinex(&cfg, data_path(COD_CLK), load_rinex(COD_CLK))
            .unwrap();
    }

    let a = QcIndexing::Custom("A".to_string());
    let b = QcIndexing::Custom("B".to_string());

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(QcClockResidualProcessor::new(
            QcClockResidualConfig::clock_products(a.clone(), b.clone())
                .with_sampling(Duration::from_seconds(3600.0)),
        ))
        .build()
        .unwrap());

    let stats = clock_residual_stats(&results);

    // stations: 300" sampling (CODE MGEX), satellites: 30"
    let (stations, raw, aligned, _) = stats["station"];
    assert!(stations > 1000, "{}", stations);
    assert_eq!(raw, 0.0);
    assert_eq!(aligned, 0.0);

    let (satellites, raw, _, _) = stats["GPS"];
    assert!(satellites > 500, "{}", satellites);
    assert_eq!(raw, 0.0);

    // a product compared to itself is skipped
    let mut products = QcInputProducts::default();
    products
        .load_rinex(
            &QcInputConfig::default(),
            data_path(COD_CLK),
            load_rinex(COD_CLK),
        )
        .unwrap();

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(QcClockResidualProcessor::new(QcClockResidualConfig::new(
            QcClockProduct::Clock(None),
            QcClockProduct::Clock(None),
        )))
        .build()
        .unwrap());

    assert!(results.is_empty());
}

/// Precise clocks - broadcast clocks (ESBC00DNK NAV): broadcast clock quality
#[test]
fn clock_residuals_broadcast() {
    let products = load_products(
        &QcInputConfig::default(),
        &[GRG_CLK, "NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz"],
    );

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(QcClockResidualProcessor::new(
            QcClockResidualConfig::new(
                QcClockProduct::Clock(None),
                QcClockProduct::Broadcast(None),
            )
            .with_sampling(Duration::from_seconds(900.0)),
        ))
        .build()
        .unwrap());

    let stats = clock_residual_stats(&results);

    let (n, _, aligned, _) = stats["GPS"];
    assert!(n > 1500, "{}", n);
    assert!(aligned > 0.5 && aligned < 5.0, "{}", aligned);

    let (_, _, aligned, _) = stats["Galileo"];
    assert!(aligned < 2.0, "{}", aligned);
}
