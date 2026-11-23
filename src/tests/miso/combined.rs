//! Combinations of identical sources: 48h of the same receiver, 48h of ephemerides.
use super::*;

const NYA1_DAY1: &str = "CRNX/V3/NYA100NOR_S_20241270000_01D_30S_MO.crx.gz";
const NYA1_DAY2: &str = "CRNX/V3/NYA100NOR_S_20241280000_01D_30S_MO.crx.gz";

const NYA1_NAV: [&str; 4] = [
    "NAV/V3/NYA100NOR_S_20241270000_01D_GN.rnx.gz",
    "NAV/V3/NYA100NOR_S_20241270000_01D_EN.rnx.gz",
    "NAV/V3/NYA100NOR_S_20241280000_01D_GN.rnx.gz",
    "NAV/V3/NYA100NOR_S_20241280000_01D_EN.rnx.gz",
];

/// Known content of the 2 NYA1 files (verified against the files in the single tests)
const NYA1_48H_SIGNALS: usize = 1_754_604 + 1_746_168;

fn nya1() -> QcIndexing {
    QcIndexing::Receiver("TRIMBLE NETR9 #5207K82137".to_string())
}

fn epoch(s: &str) -> Epoch {
    Epoch::from_str(s).unwrap()
}

/// Verifies a continuous 48h run of the NYA1 receiver
fn assert_nya1_48h(results: &RunResults) {
    assert_eq!(results.epochs.len(), 2 * 2880);
    assert_eq!(results.signals, NYA1_48H_SIGNALS);
    assert_eq!(results.stats.late_items, 0);

    assert_eq!(
        results.epochs.first().unwrap().epoch,
        epoch("2024-05-06T00:00:00 GPST")
    );
    assert_eq!(
        results.epochs.last().unwrap().epoch,
        epoch("2024-05-07T23:59:30 GPST")
    );

    // continuous 30s sampling, day boundary included
    for pair in results.epochs.windows(2) {
        assert_eq!(
            pair[1].epoch - pair[0].epoch,
            Duration::from_seconds(30.0),
            "gap at {}",
            pair[0].epoch
        );
    }

    for summary in results.epochs.iter() {
        assert_eq!(summary.receivers.len(), 1);
        assert_eq!(summary.receivers[0].receiver, nya1());
    }
}

/// Both files of the same receiver are merged by the input products table
/// (same indexing): the engine sees a single 48h signal source.
#[test]
fn crinex_48h_merged() {
    let products = load_products(&QcInputConfig::default(), &[NYA1_DAY1, NYA1_DAY2]);

    assert_eq!(
        products.obs_rinex_iter().count(),
        1,
        "files should be merged"
    );

    let engine = QcEngine::<TestOutput>::builder()
        .with_products(products)
        .with_processor(QcSignalProcessor)
        .with_processor(EpochRecorder::default())
        .build()
        .unwrap();

    assert_nya1_48h(&run(engine));
}

/// Same 48h, but streamed: files are chained and opened lazily, so only one day is
/// held in memory at a time. The second file must not be opened before the first one
/// has been entirely processed.
#[test]
fn crinex_48h_chained() {
    let day2_opened = Arc::new(AtomicBool::new(false));

    let lazy_file = |file: &'static str, opened: Option<Arc<AtomicBool>>| {
        QcLazySource::new(QcSourceDescriptor::new(nya1()).with_signals(), move || {
            if let Some(opened) = opened {
                opened.store(true, Ordering::SeqCst);
            }
            let source = QcRinexObsSource::new(nya1(), load_rinex(file))?;
            Ok(Box::new(source) as Box<dyn QcSource>)
        })
    };

    let chain = QcChainSource::new(
        QcSourceDescriptor::new(nya1()).with_signals(),
        vec![
            Box::new(lazy_file(NYA1_DAY1, None)),
            Box::new(lazy_file(NYA1_DAY2, Some(day2_opened.clone()))),
        ],
    );

    // the engine needs to look past the last epoch of day 1 to know it is complete:
    // day 2 is opened while day 1's last epoch is being completed, not before.
    let recorder = EpochRecorder {
        last: None,
        not_before: Some((day2_opened.clone(), epoch("2024-05-06T23:59:30 GPST"))),
    };

    let engine = QcEngine::<TestOutput>::builder()
        .with_source(chain)
        .with_processor(QcSignalProcessor)
        .with_processor(recorder)
        .build()
        .unwrap();

    let results = run(engine);

    assert!(day2_opened.load(Ordering::SeqCst));
    assert_nya1_48h(&results);
}

/// Two identical signal sources cannot be stacked as is: they must be merged
/// (input products table) or chained (streaming).
#[test]
fn duplicate_receiver_is_rejected() {
    let empty = |name: &str| {
        QcIterSource::new(
            QcSourceDescriptor::new(QcIndexing::Receiver(name.to_string())).with_signals(),
            std::iter::empty(),
        )
    };

    let engine = QcEngine::<TestOutput>::builder()
        .with_source(empty("NYA1"))
        .with_source(empty("NYA1"))
        .build();

    assert!(matches!(
        engine,
        Err(QcEngineError::DuplicateSignalSource(_))
    ));
}

/// 48h of observations + 48h of ephemerides (GPS + Galileo, 4 files merged in a
/// single source thanks to a custom indexing): every satellite in sight remains covered,
/// across the day boundary, while the ephemeris buffer remains small.
/// Then, the same observations with only day 1 ephemerides: day 2 is not covered.
#[test]
fn crinex_48h_with_nav_48h() {
    let cfg = QcInputConfig {
        nav_rinex_indexing: Some(QcIndexingConfig::Custom("NYA1-BRDC".to_string())),
        ..Default::default()
    };

    let constellations = [Constellation::GPS, Constellation::Galileo];

    let mut files = vec![NYA1_DAY1, NYA1_DAY2];
    files.extend_from_slice(&NYA1_NAV);

    let products = load_products(&cfg, &files);
    assert_eq!(products.obs_rinex_iter().count(), 1);
    assert_eq!(
        products.nav_rinex_iter().count(),
        1,
        "NAV files should be merged"
    );

    let engine = QcEngine::<TestOutput>::builder()
        .with_products(products)
        .with_processor(EpochRecorder::default())
        .with_processor(CoverageCounter::new(&constellations))
        .with_processor(CoverageCounter::new(&constellations).within(
            epoch("2024-05-06T23:00:00 GPST"),
            epoch("2024-05-07T01:00:00 GPST"),
        ))
        .build()
        .unwrap();

    let results = run(engine);
    let coverage = results.coverage[0];
    let boundary = results.coverage[1];

    assert_eq!(results.epochs.len(), 2 * 2880);
    let max_buffered = results
        .epochs
        .iter()
        .map(|e| e.buffered_ephemerides)
        .max()
        .unwrap();

    assert!(coverage.ratio() > 0.999, "48h coverage: {:?}", coverage);

    // bounded memory: a fraction of the ~1870 ephemerides of the session,
    // and no growth from one day to the other
    let max_buffered_day = |day: usize| {
        results.epochs[day * 2880..(day + 1) * 2880]
            .iter()
            .map(|e| e.buffered_ephemerides)
            .max()
            .unwrap()
    };

    assert!(
        max_buffered < 300,
        "ephemeris buffer is too large: {}",
        max_buffered
    );
    assert!(
        max_buffered_day(1) <= max_buffered_day(0) + max_buffered_day(0) / 10,
        "ephemeris buffer grows: day1={} day2={}",
        max_buffered_day(0),
        max_buffered_day(1)
    );

    // across the day boundary: no coverage loss
    assert!(boundary.covered > 0);
    assert_eq!(boundary.uncovered, 0, "coverage loss at day boundary");

    // only day 1 ephemerides
    let mut files = vec![NYA1_DAY1, NYA1_DAY2];
    files.extend_from_slice(&NYA1_NAV[..2]);

    let products = load_products(&cfg, &files);

    let engine = QcEngine::<TestOutput>::builder()
        .with_products(products)
        .with_processor(CoverageCounter::new(&constellations))
        .build()
        .unwrap();

    let day1_only = run(engine).coverage[0];

    assert!(
        day1_only.ratio() < 0.6,
        "day 2 should not be covered: {:?}",
        day1_only
    );
}
