//! SP3 indexing preferences
use crate::{prelude::*, tests::miso::data_path};

const GRG_176: &str = "SP3/C/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3.gz";
const GRG_177: &str = "SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz";
const IAC_177: &str = "SP3/D/Sta21114.sp3.gz";

fn load(cfg: &QcInputConfig, files: &[&str]) -> QcInputProducts {
    let mut products = QcInputProducts::default();
    for file in files {
        products.load_gzip_file(cfg, data_path(file)).unwrap();
    }
    products
}

fn keys(products: &QcInputProducts) -> Vec<QcIndexing> {
    let mut keys = products
        .sp3_iter()
        .map(|(k, _)| k.clone())
        .collect::<Vec<_>>();
    keys.sort_by_key(|k| k.to_string());
    keys
}

fn sp3_cfg(indexing: QcIndexingConfig) -> QcInputConfig {
    QcInputConfig {
        sp3_indexing: Some(indexing),
        ..Default::default()
    }
}

/// Default: indexed by agency, two daily files of the same agency are merged
#[test]
fn sp3_default_indexing() {
    let products = load(&QcInputConfig::default(), &[GRG_176, GRG_177, IAC_177]);

    assert_eq!(
        keys(&products),
        vec![
            QcIndexing::Agency("GRGS".to_string()),
            QcIndexing::Agency("IAC".to_string()),
        ]
    );

    let (_, grg) = products
        .sp3_iter()
        .find(|(k, _)| **k == QcIndexing::Agency("GRGS".to_string()))
        .unwrap();

    // 2 days of 15' samples
    let mut epochs = grg.data.keys().map(|k| k.epoch).collect::<Vec<_>>();
    epochs.sort();
    epochs.dedup();
    assert_eq!(epochs.len(), 2 * 96);
}

/// No preference: automatic indexing (agency)
#[test]
fn sp3_automatic_indexing() {
    let cfg = QcInputConfig {
        sp3_indexing: None,
        ..Default::default()
    };

    let products = load(&cfg, &[GRG_177]);
    assert_eq!(
        keys(&products),
        vec![QcIndexing::Agency("GRGS".to_string())]
    );
}

/// File name: one product per file, even from the same agency
#[test]
fn sp3_file_name_indexing() {
    let products = load(
        &sp3_cfg(QcIndexingConfig::FileName),
        &[GRG_176, GRG_177, IAC_177],
    );

    assert_eq!(
        keys(&products),
        vec![
            QcIndexing::Custom("GRG0MGXFIN_20201760000_01D_15M_ORB".to_string()),
            QcIndexing::Custom("GRG0MGXFIN_20201770000_01D_15M_ORB".to_string()),
            QcIndexing::Custom("Sta21114".to_string()),
        ]
    );

    // uncompressed file
    let mut products = QcInputProducts::default();
    products
        .load_file(
            &sp3_cfg(QcIndexingConfig::FileName),
            data_path("SP3/C/co108870.sp3"),
        )
        .unwrap();

    assert_eq!(
        keys(&products),
        vec![QcIndexing::Custom("co108870".to_string())]
    );
}

/// Custom alias: files gathered under one name
#[test]
fn sp3_custom_indexing() {
    let products = load(
        &sp3_cfg(QcIndexingConfig::Custom("GRG-FINAL".to_string())),
        &[GRG_176, GRG_177],
    );

    assert_eq!(
        keys(&products),
        vec![QcIndexing::Custom("GRG-FINAL".to_string())]
    );
}

/// Preferences that SP3 files cannot describe fall back to automatic indexing
#[test]
fn sp3_not_applicable_indexing() {
    for preference in [
        QcIndexingConfig::Receiver,
        QcIndexingConfig::Antenna,
        QcIndexingConfig::Operator,
    ] {
        let products = load(&sp3_cfg(preference), &[IAC_177]);
        assert_eq!(keys(&products), vec![QcIndexing::Agency("IAC".to_string())]);
    }
}

/// The SP3 indexing is the identity of the precise products in the engine:
/// one file indexed by its name, compared to an agency.
#[test]
fn sp3_indexing_in_engine() {
    let mut products = QcInputProducts::default();

    products
        .load_gzip_file(&sp3_cfg(QcIndexingConfig::FileName), data_path(GRG_177))
        .unwrap();

    products
        .load_gzip_file(&QcInputConfig::default(), data_path(IAC_177))
        .unwrap();

    let grg = QcIndexing::Custom("GRG0MGXFIN_20201770000_01D_15M_ORB".to_string());
    let iac = QcIndexing::Agency("IAC".to_string());

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_processor(QcOrbitResidualProcessor::new(
            QcOrbitResidualConfig::precise(grg.clone(), iac.clone())
                .with_constellations(&[Constellation::GPS]),
        ))
        .build()
        .unwrap();

    let mut residuals = 0;

    for item in engine {
        if let QcOutput::OrbitResidual(r) = item.unwrap().payload {
            assert_eq!(r.lhs_source, grg);
            assert_eq!(r.rhs_source, iac);
            assert!(r.norm_m() < 0.25);
            residuals += 1;
        }
    }

    assert!(residuals > 2000, "{}", residuals);
}
