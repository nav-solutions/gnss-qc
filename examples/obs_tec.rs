//! TEC estimated from the signal observations of a network of receivers
//! (ESBC00DNK and MOJN00DNK, 2020-06-25, broadcast ephemerides), at the Ionospheric Pierce
//! Point of each line of sight: one point per epoch, receiver and satellite, as CSV on stdout.
//! The leveling of each phase arc is printed on stderr.
//!
//! These points are the raw material of TEC maps: grid them (IONEX) in your application.
//!
//! ```bash
//! cargo run --release --features ionex --example obs_tec > tec.csv
//! ```
#[path = "common/mod.rs"]
mod common;

use gnss_qc::prelude::*;

fn main() {
    let products = common::load_products(
        &QcInputConfig::default(),
        &[
            "CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz",
            "CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz",
            "NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz",
        ],
    );

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_processor(QcObsTecProcessor::new(QcObsTecConfig::default()))
        .build()
        .unwrap();

    println!("epoch,receiver,sv,latitude_deg,longitude_deg,elevation_deg,stec_tecu,vtec_tecu,arc");

    for item in engine {
        let item = item.unwrap();
        match item.payload {
            QcOutput::TecEstimate(e) => {
                println!(
                    "{},{},{},{:.4},{:.4},{:.2},{:.3},{:.3},{}",
                    item.epoch,
                    e.receiver,
                    e.sv,
                    e.latitude_deg,
                    e.longitude_deg,
                    e.elevation_deg,
                    e.slant_tecu,
                    e.vertical_tecu,
                    e.arc
                );
            }
            QcOutput::TecArc(arc) => {
                eprintln!(
                    "{} {} arc #{}: {} - {}, {} samples, leveling {:.2} TECu, code noise {:.2} TECu",
                    arc.receiver,
                    arc.sv,
                    arc.arc,
                    arc.start,
                    arc.end,
                    arc.samples,
                    arc.leveling_tecu,
                    arc.code_noise_tecu
                );
            }
            _ => {}
        }
    }
}
