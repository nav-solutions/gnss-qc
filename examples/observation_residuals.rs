//! Observation residuals between two receivers sharing the same antenna (zero baseline):
//! a geodetic receiver (Septentrio PolaRx5) and a software receiver (GNSS-SDR),
//! 2023-12-18, ~2 hours. Pseudo range and phase residuals (A - B) of the common observables,
//! for each satellite: they describe the relative behavior of both receivers (clocks).
//!
//! ```bash
//! cargo run --release --example observation_residuals > residuals.csv
//! ```
#[path = "common/mod.rs"]
mod common;

use gnss_qc::prelude::*;

fn main() {
    let products = common::load_products(
        &QcInputConfig::default(),
        &["OBS/V3/OB713520.23O.gz", "OBS/V3/gps.23O.gz"],
    );

    // A - B: PolaRx5 - GNSS-SDR (pseudo range and phase, by default)
    let residuals = QcObservationResidualProcessor::new(QcObservationResidualConfig::new(
        QcIndexing::Receiver("SEPT POLARX5TR #3088149".to_string()),
        QcIndexing::Receiver("Software Receiver #GNSS-SDR".to_string()),
    ));

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_processor(residuals)
        .build()
        .unwrap();

    println!("epoch,rhs_epoch,sv,observable,residual");

    for item in engine {
        let item = item.unwrap();
        if let QcOutput::ObservationResidual(r) = item.payload {
            println!(
                "{},{},{},{},{:.3}",
                item.epoch, r.rhs_epoch, r.sv, r.observable, r.residual
            );
        }
    }
}
