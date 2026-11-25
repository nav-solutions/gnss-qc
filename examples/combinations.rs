//! Signal combinations of a geodetic receiver (ESBC00DNK, Septentrio PolaRx5, 2020-06-25):
//! Geometry Free, Ionosphere Free (code and phase) and code multipath, for every pair of
//! frequencies, as CSV on stdout. The code multipath statistics of each phase arc are printed
//! on stderr.
//!
//! ```bash
//! cargo run --release --example combinations > combinations.csv
//! ```
#[path = "common/mod.rs"]
mod common;

use gnss_qc::prelude::*;

fn main() {
    let products = common::load_products(
        &QcInputConfig::default(),
        &["CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz"],
    );

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_processor(QcCombinationProcessor::new(QcCombinationConfig::default()))
        .build()
        .unwrap();

    println!("epoch,sv,combination,lhs,rhs,value_m,arc,debiased_m");

    for item in engine {
        let item = item.unwrap();
        match item.payload {
            QcOutput::Combination(c) => {
                println!(
                    "{},{},{},{},{},{:.4},{},{}",
                    item.epoch,
                    c.sv,
                    c.kind,
                    c.lhs,
                    c.rhs,
                    c.value_m,
                    c.arc.map(|arc| arc.to_string()).unwrap_or_default(),
                    c.debiased_m
                        .map(|value| format!("{:.4}", value))
                        .unwrap_or_default(),
                );
            }
            QcOutput::MultipathArc(arc) => {
                eprintln!(
                    "{} MP {} ({}) arc #{}: {} - {}, {} samples, RMS {:.3} m",
                    arc.sv,
                    arc.code,
                    arc.phase,
                    arc.arc,
                    arc.start,
                    arc.end,
                    arc.samples,
                    arc.rms_m
                );
            }
            _ => {}
        }
    }
}
