//! Precise clocks of a Clock RINEX (CODE MGEX final products, 2024-05-07):
//! onboard clocks of the satellites and clocks of the ground stations, every 5'.
//!
//! ```bash
//! cargo run --release --example precise_clocks > clocks.csv
//! ```
#[path = "common/mod.rs"]
mod common;

use gnss_qc::prelude::*;

fn main() {
    let products = common::load_products(
        &QcInputConfig::default(),
        &["CLK/V3/COD0MGXFIN_20241280000_01D_30S_CLK.CLK.gz"],
    );

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_processor(QcPreciseClockProcessor::new(
            QcPreciseClockConfig::default().with_sampling(Duration::from_seconds(300.0)),
        ))
        .build()
        .unwrap();

    println!("epoch,agency,clock,kind,offset_us");

    for item in engine {
        let item = item.unwrap();
        if let QcOutput::PreciseClock(clock) = item.payload {
            let kind = match clock.state.clock {
                QcClockId::Satellite(_) => "satellite",
                QcClockId::Station(_) => "station",
            };

            println!(
                "{},{},{},{},{:.6}",
                item.epoch,
                clock.source,
                clock.state.clock,
                kind,
                clock.state.offset_s * 1.0E6
            );
        }
    }
}
