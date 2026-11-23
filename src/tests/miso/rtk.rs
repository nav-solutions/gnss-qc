//! Two different receivers processed simultaneously (RTK compatible setup).
use super::*;

const ESBC: &str = "CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz";
const MOJN: &str = "CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz";
const ESBC_NAV: &str = "NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz";
const MOJN_NAV: &str = "NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz";

fn esbc() -> QcIndexing {
    QcIndexing::Receiver("SEPT POLARX5 #3047937".to_string())
}

fn mojn() -> QcIndexing {
    QcIndexing::Receiver("SEPT POLARX5 #3022663".to_string())
}

/// Counts, at each epoch, the GPS satellites seen by both receivers
/// and covered by a valid ephemeris (what a double difference solver needs).
pub struct CommonView {
    min: usize,
}

impl QcProcessor<TestOutput> for CommonView {
    fn name(&self) -> &str {
        "common-view"
    }

    fn process(&mut self, ctx: &QcEpochContext, _: &mut QcOutputQueue<TestOutput>) {
        assert_eq!(
            ctx.receivers.len(),
            2,
            "{}: both receivers expected",
            ctx.epoch
        );

        let svs = |rx: &QcEpochSignals| {
            let mut svs = rx
                .signals
                .iter()
                .map(|sig| sig.sv)
                .filter(|sv| sv.constellation == Constellation::GPS)
                .collect::<Vec<_>>();
            svs.sort();
            svs.dedup();
            svs
        };

        let (a, b) = (svs(&ctx.receivers[0]), svs(&ctx.receivers[1]));

        let common = a
            .iter()
            .filter(|sv| b.contains(sv))
            .filter(|sv| ctx.ephemerides.select(**sv, ctx.epoch).is_some())
            .count();

        self.min = self.min.min(common);
    }

    fn finalize(&mut self, output: &mut QcOutputQueue<TestOutput>) {
        output.push(
            Epoch::default(),
            TestOutput::Coverage(CoverageStats {
                covered: self.min,
                uncovered: 0,
            }),
        );
    }
}

/// Both stations use the same receiver model: they must remain two distinct
/// signal sources (indexing includes the serial number), and each epoch presents
/// both receivers with their own, untouched, signals.
#[test]
fn rtk_two_receivers() {
    let products = load_products(&QcInputConfig::default(), &[ESBC, MOJN]);

    let mut indexes = products
        .obs_rinex_iter()
        .map(|(idx, _)| idx.clone())
        .collect::<Vec<_>>();
    indexes.sort_by_key(|idx| idx.to_string());

    assert_eq!(indexes, vec![mojn(), esbc()], "stations must not be merged");

    let engine = QcEngine::<TestOutput>::builder()
        .with_products(products)
        .with_processor(QcSignalProcessor)
        .with_processor(EpochRecorder::default())
        .build()
        .unwrap();

    let results = run(engine);

    // both files are aligned: 2880 epochs, each with 2 receivers
    assert_eq!(results.epochs.len(), 2880);
    assert_eq!(results.stats.late_items, 0);

    for summary in results.epochs.iter() {
        let mut receivers = summary
            .receivers
            .iter()
            .map(|rx| rx.receiver.clone())
            .collect::<Vec<_>>();
        receivers.sort_by_key(|idx| idx.to_string());
        assert_eq!(receivers, vec![mojn(), esbc()], "{}", summary.epoch);
    }

    // signals are not mixed between receivers
    assert_eq!(results.signals_per_receiver[&esbc()], 1_657_584);
    assert_eq!(results.signals_per_receiver[&mojn()], 1_812_457);
    assert_eq!(results.signals, 1_657_584 + 1_812_457);
}

/// Both stations + broadcast ephemerides: at every epoch, enough common satellites
/// with a valid ephemeris to form double differences (RTK compatible).
#[test]
fn rtk_common_view() {
    let cfg = QcInputConfig {
        nav_rinex_indexing: Some(QcIndexingConfig::Custom("DNK-BRDC".to_string())),
        ..Default::default()
    };

    let products = load_products(&cfg, &[ESBC, MOJN, ESBC_NAV, MOJN_NAV]);

    assert_eq!(products.obs_rinex_iter().count(), 2);
    assert_eq!(products.nav_rinex_iter().count(), 1);

    let engine = QcEngine::<TestOutput>::builder()
        .with_products(products)
        .with_processor(CommonView { min: usize::MAX })
        .build()
        .unwrap();

    let results = run(engine);

    assert_eq!(results.stats.epochs, 2880);

    let min_common = results.coverage[0].covered;
    assert!(
        min_common >= 5,
        "not enough common satellites for RTK: {}",
        min_common
    );
}
