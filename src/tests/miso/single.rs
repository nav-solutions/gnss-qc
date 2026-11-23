//! One file in, one stream out.
use std::collections::HashSet;

use super::*;

/// Single (uncompressed) Observation RINEX: every epoch and every signal
/// goes through the engine, in order, identified by its receiver.
#[test]
fn single_obs_file() {
    let file = "OBS/V3/ACOR00ESP_R_20213550000_01D_30S_MO.rnx";
    let truth = ObsTruth::new(&load_rinex(file));

    let products = load_products(&QcInputConfig::default(), &[file]);
    let receiver = QcIndexing::Receiver("LEICA GR50 #1833574".to_string());

    let engine = QcEngine::<TestOutput>::builder()
        .with_products(products)
        .with_processor(QcSignalProcessor)
        .with_processor(EpochRecorder::default())
        .build()
        .unwrap();

    let results = run(engine);

    assert_eq!(truth.epochs.len(), 25);
    assert_eq!(truth.signals, 9036);
    assert_matches_truth(&results, &truth, &receiver);
    assert_eq!(results.stats.epochs, 25);
}

/// Single CRINEX (compressed, gzipped): same guarantees, one full day (2880 epochs).
#[test]
fn single_crinex_file() {
    let file = "CRNX/V3/NYA100NOR_S_20241270000_01D_30S_MO.crx.gz";
    let truth = ObsTruth::new(&load_rinex(file));

    let products = load_products(&QcInputConfig::default(), &[file]);
    let receiver = QcIndexing::Receiver("TRIMBLE NETR9 #5207K82137".to_string());

    let engine = QcEngine::<TestOutput>::builder()
        .with_products(products)
        .with_processor(QcSignalProcessor)
        .with_processor(EpochRecorder::default())
        .build()
        .unwrap();

    let results = run(engine);

    assert_eq!(truth.epochs.len(), 2880);
    assert_eq!(truth.signals, 1_754_604);
    assert_matches_truth(&results, &truth, &receiver);

    assert_eq!(
        results.epochs.first().unwrap().epoch,
        Epoch::from_str("2024-05-06T00:00:00 GPST").unwrap()
    );
    assert_eq!(
        results.epochs.last().unwrap().epoch,
        Epoch::from_str("2024-05-06T23:59:30 GPST").unwrap()
    );
}

/// Runs a navigation file alone (context only session) and verifies that
/// every ephemeris frame goes through the engine exactly once.
fn single_nav_file(file: &str, expected: usize, constellations: usize) {
    let rinex = load_rinex(file);

    let truth = rinex
        .record
        .as_nav()
        .unwrap()
        .iter()
        .filter_map(|(k, frame)| frame.as_ephemeris().map(|_| (k.sv, k.epoch)))
        .collect::<HashSet<_>>();

    assert_eq!(truth.len(), expected);

    let products = load_products(&QcInputConfig::default(), &[file]);

    let engine = QcEngine::<TestOutput>::builder()
        .with_products(products)
        .with_processor(QcEphemerisProcessor::default())
        .build()
        .unwrap();

    let results = run(engine);

    // exactly once
    let streamed = results
        .ephemerides
        .iter()
        .map(|eph| (eph.sv, eph.toc))
        .collect::<HashSet<_>>();

    assert_eq!(results.ephemerides.len(), expected, "duplicates or losses");
    assert_eq!(streamed, truth);

    // epochs are driven by the ToC of the ephemerides
    let tocs = truth.iter().map(|(_, toc)| *toc).collect::<HashSet<_>>();
    assert_eq!(results.stats.epochs, tocs.len());

    let nb_constellations = streamed
        .iter()
        .map(|(sv, _)| sv.constellation)
        .collect::<HashSet<_>>()
        .len();
    assert_eq!(nb_constellations, constellations);
}

/// Single NAV file (GPS only)
#[test]
fn single_nav_file_gps() {
    single_nav_file("NAV/V3/NYA100NOR_S_20241270000_01D_GN.rnx.gz", 217, 1);
}

/// Single NAV file, mixed constellations and timescales (GPST, GST, BDT...)
#[test]
fn single_nav_file_mixed() {
    single_nav_file("NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz", 4092, 8);
}
