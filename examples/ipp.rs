//! Ionospheric Pierce Points (IPP): where the lines of sight of a receiver pierce the
//! ionosphere (single layer model, 450 km), over time.
//! Satellites positions: precise products (SP3) whenever available, broadcast otherwise.
//!
//! Station ESBC00DNK (Denmark), 2020-06-25 00:00-01:00.
//!
//! ```bash
//! cargo run --release --features ionex --example ipp > ipp.csv
//! ```
#[path = "common/mod.rs"]
mod common;

use std::str::FromStr;

use gnss_qc::prelude::*;

fn main() {
    let products = common::load_products(
        &QcInputConfig::default(),
        &[
            "CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz",
            "NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz",
            "SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz",
        ],
    );

    let ipp = QcIppProcessor::new(QcIppConfig {
        orbits: QcOrbitPreference::Auto,
        elevation_mask_deg: 10.0,
        shell_altitude_km: 450.0,
        ..Default::default()
    });

    let t0 = Epoch::from_str("2020-06-25T00:00:00 GPST").unwrap();

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_time_window(Some(t0), Some(t0 + Duration::from_hours(1.0)))
        .with_processor(ipp)
        .build()
        .unwrap();

    println!("epoch,sv,orbit,elevation_deg,azimuth_deg,ipp_lat_deg,ipp_long_deg,obliquity");

    for item in engine {
        let item = item.unwrap();
        if let QcOutput::IPP(ipp) = item.payload {
            println!(
                "{},{},{:?},{:.3},{:.3},{:.4},{:.4},{:.4}",
                item.epoch,
                ipp.sv,
                ipp.orbit,
                ipp.elevation_deg,
                ipp.azimuth_deg,
                ipp.latitude_deg,
                ipp.longitude_deg,
                ipp.obliquity_factor
            );
        }
    }
}
