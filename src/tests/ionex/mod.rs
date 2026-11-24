//! IONEX inputs: TEC maps, analysis and temporal interpolation.
use std::collections::HashMap;

use ionex::prelude::{TemporalInterpolation, IONEX};

use crate::{
    prelude::*,
    tests::miso::{data_path, load_products},
};

const CKMG_2009: &str = "IONEX/V1/CKMG0080.09I.gz";
const CKMG_2021: &str = "IONEX/V1/CKMG0090.21I.gz";

fn run(engine: QcEngine<QcOutput>) -> Vec<(Epoch, QcOutput)> {
    engine
        .map(|item| {
            let item = item.unwrap();
            (item.epoch, item.payload)
        })
        .collect()
}

fn points(results: &[(Epoch, QcOutput)]) -> Vec<(Epoch, &QcTecMapPoint)> {
    results
        .iter()
        .filter_map(|(t, output)| match output {
            QcOutput::TecPoint(point) => Some((*t, point)),
            _ => None,
        })
        .collect()
}

fn statistics(results: &[(Epoch, QcOutput)]) -> Vec<(Epoch, &QcTecMapStatistics)> {
    results
        .iter()
        .filter_map(|(t, output)| match output {
            QcOutput::TecMap(stats) => Some((*t, stats)),
            _ => None,
        })
        .collect()
}

fn key(t: Epoch, lat: f64, lon: f64) -> (Epoch, i64, i64) {
    (
        t,
        (lat * 1.0E3).round() as i64,
        (lon * 1.0E3).round() as i64,
    )
}

/// IONEX are indexed by agency ("RUN BY"); maps delivered node after node
#[test]
fn ionex_products() {
    let products = load_products(&QcInputConfig::default(), &[CKMG_2009, CKMG_2021]);

    // both files: same agency, gathered
    let entries = products
        .ionex_iter()
        .map(|(k, ionex)| (k.clone(), ionex.epoch_iter().count()))
        .collect::<Vec<_>>();

    assert_eq!(
        entries,
        vec![(QcIndexing::Agency("AIUB".to_string()), 13 + 25)]
    );

    // by file name
    let cfg = QcInputConfig {
        ionex_indexing: Some(QcIndexingConfig::FileName),
        ..Default::default()
    };

    let products = load_products(&cfg, &[CKMG_2009, CKMG_2021]);
    let mut names = products
        .ionex_iter()
        .map(|(k, _)| k.to_string())
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(names, vec!["CKMG0080", "CKMG0090"]);
}

/// Generic loaders identify IONEX files (and still identify SP3 files)
#[test]
fn ionex_generic_loading() {
    let mut products = QcInputProducts::default();
    let cfg = QcInputConfig::default();

    products.load_gzip_file(&cfg, data_path(CKMG_2009)).unwrap();

    #[cfg(feature = "sp3")]
    products
        .load_gzip_file(
            &cfg,
            data_path("SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz"),
        )
        .unwrap();

    assert_eq!(products.ionex_iter().count(), 1);

    #[cfg(feature = "sp3")]
    {
        assert_eq!(products.sp3_iter().count(), 1);
        assert_eq!(
            products.sp3_iter().next().unwrap().0,
            &QcIndexing::Agency("GRGS".to_string())
        );
    }
}

/// Native maps: every node is streamed as is, statistics per region
#[test]
fn tec_native_maps() {
    let ionex = IONEX::from_gzip_file(data_path(CKMG_2009)).unwrap();

    let expected = ionex
        .record
        .iter()
        .map(|(k, tec)| {
            (
                key(k.epoch, k.latitude_ddeg(), k.longitude_ddeg()),
                tec.tecu(),
            )
        })
        .collect::<HashMap<_, _>>();

    let products = load_products(&QcInputConfig::default(), &[CKMG_2009]);

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(QcTecProcessor::new(QcTecConfig::default().with_points()))
        .build()
        .unwrap());

    let points = points(&results);
    assert_eq!(points.len(), expected.len());
    assert_eq!(points.len(), 13 * 71 * 73);

    for (t, point) in points.iter() {
        assert!(!point.interpolated);
        assert_eq!(point.source, QcIndexing::Agency("AIUB".to_string()));
        let tecu = expected[&key(*t, point.point.latitude_deg, point.point.longitude_deg)];
        assert_eq!(point.point.tecu, tecu);
    }

    // statistics: 13 maps, 6 regions
    let stats = statistics(&results);
    assert_eq!(stats.len(), 13 * 6);

    for (t, s) in stats.iter() {
        let selected = expected
            .iter()
            .filter(|((e, lat, _), _)| {
                let lat = *lat as f64 / 1.0E3;
                e == t
                    && match s.region.as_str() {
                        "Global" => true,
                        "Equatorial" => (-30.0..=30.0).contains(&lat),
                        "North high latitudes" => lat >= 60.0,
                        _ => return false,
                    }
            })
            .map(|(_, tecu)| *tecu)
            .collect::<Vec<_>>();

        if selected.is_empty() {
            continue;
        }

        let mean = selected.iter().sum::<f64>() / selected.len() as f64;
        assert_eq!(s.nodes, selected.len());
        assert!((s.mean_tecu - mean).abs() < 1.0E-9);
        assert!(s.min_tecu <= s.mean_tecu + 1.0E-9 && s.mean_tecu <= s.max_tecu + 1.0E-9);
    }
}

/// Temporal interpolation (x2): identical to the IONEX library resampling, for the
/// three strategies; native maps are preserved
#[test]
fn tec_temporal_interpolation() {
    let ionex = IONEX::from_gzip_file(data_path(CKMG_2009)).unwrap();

    for (method, reference) in [
        (
            QcTecInterpolation::RotatedMaps,
            TemporalInterpolation::RotatedMaps,
        ),
        (QcTecInterpolation::Linear, TemporalInterpolation::Linear),
        (
            QcTecInterpolation::NearestMap,
            TemporalInterpolation::NearestMap,
        ),
    ] {
        // 2h maps to 1h maps
        let resampled = ionex
            .temporally_resampled(Duration::from_hours(1.0), reference)
            .unwrap();

        let expected = resampled
            .record
            .iter()
            .map(|(k, tec)| {
                (
                    key(k.epoch, k.latitude_ddeg(), k.longitude_ddeg()),
                    tec.tecu(),
                )
            })
            .collect::<HashMap<_, _>>();

        let products = load_products(&QcInputConfig::default(), &[CKMG_2009]);

        let results = run(QcEngine::builder()
            .with_products(products)
            .with_processor(QcTecProcessor::new(
                QcTecConfig::default()
                    .with_upsampling_factor(2.0)
                    .with_interpolation(method)
                    .with_points(),
            ))
            .build()
            .unwrap());

        let points = points(&results);

        // 25 maps (13 native + 12 interpolated)
        let epochs = points
            .iter()
            .map(|(t, _)| *t)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(epochs.len(), 25, "{:?}", method);
        assert_eq!(points.len(), expected.len(), "{:?}", method);

        let mut interpolated = 0;

        for (t, point) in points.iter() {
            let reference = expected[&key(*t, point.point.latitude_deg, point.point.longitude_deg)];
            assert!(
                (point.point.tecu - reference).abs() < 1.0E-3,
                "{:?} {} {:?}: {} {}",
                method,
                t,
                point.point,
                point.point.tecu,
                reference
            );

            if point.interpolated {
                interpolated += 1;
            }
        }

        assert_eq!(interpolated, 12 * 71 * 73);
    }
}

/// Periodic resampling, statistics of interpolated maps lie between the surrounding maps
#[test]
fn tec_resampling_statistics() {
    let products = load_products(&QcInputConfig::default(), &[CKMG_2021]);

    let results = run(QcEngine::builder()
        .with_products(products)
        .with_processor(QcTecProcessor::new(
            QcTecConfig::default()
                .with_sampling(Duration::from_seconds(900.0))
                .with_interpolation(QcTecInterpolation::Linear)
                .with_regions(vec![QcTecRegion::global()]),
        ))
        .build()
        .unwrap());

    let stats = statistics(&results);

    // 1h maps over 24h: 97 maps every 15'
    assert_eq!(stats.len(), 97);

    let native = stats
        .iter()
        .filter(|(_, s)| !s.interpolated)
        .map(|(t, s)| (*t, s.mean_tecu))
        .collect::<std::collections::BTreeMap<_, _>>();

    assert_eq!(native.len(), 25);

    for (t, s) in stats.iter().filter(|(_, s)| s.interpolated) {
        // linear interpolation of every node: linear interpolation of the mean
        let (t0, m0) = native.range(..*t).next_back().unwrap();
        let (t1, m1) = native.range(*t..).next().unwrap();
        let w = (*t - *t0).to_seconds() / (*t1 - *t0).to_seconds();
        assert!((s.mean_tecu - (m0 + (m1 - m0) * w)).abs() < 1.0E-9);
    }
}
