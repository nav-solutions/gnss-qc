//! Orbit residuals, 2020-06-25:
//! - precise products of two agencies (GRG - IAC);
//! - broadcast ephemerides (ESBC00DNK) - precise products (GRG).
//!
//! Residuals in ECEF and in the radial / along track / cross track frame,
//! plus the clock residual.
//!
//! ```bash
//! cargo run --release --example orbit_residuals > residuals.csv
//! ```
#[path = "common/mod.rs"]
mod common;

use gnss_qc::prelude::*;

fn run(files: &[&str], cfg: QcOrbitResidualConfig) {
    let products = common::load_products(&QcInputConfig::default(), files);

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_processor(QcOrbitResidualProcessor::new(cfg))
        .build()
        .unwrap();

    for item in engine {
        let item = item.unwrap();
        if let QcOutput::OrbitResidual(r) = item.payload {
            println!(
                "{},{},{},{:?},{},{:?},{:.3},{:.3},{:.3},{:.3},{:.3}",
                item.epoch,
                r.sv,
                r.lhs_source,
                r.lhs_kind,
                r.rhs_source,
                r.rhs_kind,
                r.radial_m,
                r.along_track_m,
                r.cross_track_m,
                r.norm_m(),
                r.clock_s.unwrap_or(f64::NAN) * 1.0E9,
            );
        }
    }
}

fn main() {
    println!("epoch,sv,lhs,lhs_kind,rhs,rhs_kind,radial_m,along_m,cross_m,norm_m,clock_ns");

    // precise products of two agencies
    run(
        &[
            "SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz",
            "SP3/D/Sta21114.sp3.gz",
        ],
        QcOrbitResidualConfig::precise(
            QcIndexing::Agency("GRGS".to_string()),
            QcIndexing::Agency("IAC".to_string()),
        ),
    );

    // broadcast ephemerides - precise products, every 15', recent ephemerides only
    run(
        &[
            "NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz",
            "SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz",
        ],
        QcOrbitResidualConfig::broadcast_precise()
            .with_sampling(Duration::from_seconds(900.0))
            .with_max_broadcast_age(Duration::from_hours(1.0)),
    );
}
