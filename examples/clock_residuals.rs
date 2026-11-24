//! Clock residuals between two analysis centers, 2020-06-25:
//! GRG (Clock RINEX, 30") - IAC (SP3 clock states, 15'), every 15'.
//!
//! Each analysis center has its own clock datum: the raw residuals share a common
//! offset at each epoch, removed per constellation (aligned residuals).
//! Two Clock RINEX are compared the same way (QcClockResidualConfig::clock_products).
//!
//! ```bash
//! cargo run --release --example clock_residuals > clock_residuals.csv
//! ```
#[path = "common/mod.rs"]
mod common;

use gnss_qc::prelude::*;

fn main() {
    let products = common::load_products(
        &QcInputConfig::default(),
        &[
            "CLK/V3/GRG0MGXFIN_20201770000_01D_30S_CLK.CLK.gz",
            "SP3/D/Sta21114.sp3.gz",
        ],
    );

    let cfg = QcClockResidualConfig::new(
        QcClockProduct::Clock(Some(QcIndexing::Agency("GRG".to_string()))),
        QcClockProduct::PreciseState(Some(QcIndexing::Agency("IAC".to_string()))),
    )
    .with_sampling(Duration::from_seconds(900.0));

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_processor(QcClockResidualProcessor::new(cfg))
        .build()
        .unwrap();

    println!("epoch,clock,lhs,rhs,residual_ns,datum_ns,aligned_ns");

    for item in engine {
        let item = item.unwrap();
        if let QcOutput::ClockResidual(r) = item.payload {
            println!(
                "{},{},{},{},{:.4},{:.4},{:.4}",
                item.epoch,
                r.clock,
                r.lhs_source,
                r.rhs_source,
                r.residual_s * 1.0E9,
                r.datum_s.unwrap_or(f64::NAN) * 1.0E9,
                r.aligned_s().unwrap_or(f64::NAN) * 1.0E9,
            );
        }
    }
}
