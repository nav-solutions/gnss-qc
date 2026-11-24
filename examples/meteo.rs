//! Meteo observations of the POTS00DEU station (Potsdam, 2023-09-11, 5' sampling):
//! pressure, temperature and humidity time series.
//!
//! ```bash
//! cargo run --release --example meteo > meteo.csv
//! ```
#[path = "common/mod.rs"]
mod common;

use gnss_qc::prelude::*;

fn main() {
    let products = common::load_products(
        &QcInputConfig::default(),
        &["MET/V3/POTS00DEU_R_20232540000_01D_05M_MM.rnx.gz"],
    );

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_processor(QcMeteoProcessor::default())
        .build()
        .unwrap();

    println!("epoch,source,observable,value");

    for item in engine {
        let item = item.unwrap();
        if let QcOutput::Meteo(sample) = item.payload {
            println!(
                "{},{},{},{}",
                item.epoch, sample.source, sample.observable, sample.value
            );
        }
    }
}
