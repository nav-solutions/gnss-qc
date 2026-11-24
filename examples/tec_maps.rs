//! TEC maps (IONEX): statistics per region, and temporal interpolation.
//!
//! CKMG0080.09I (CODE, 2009-01-08) provides one worldwide map every 2 hours (13 maps).
//! The maps are interpolated in time (x2, rotated maps strategy of the IONEX
//! specifications): the nodes of every map (native and interpolated) are streamed
//! out by GNSS-Qc, and accumulated here to synthesize a new IONEX file (25 maps,
//! one every hour). Statistics of every map (per region) are printed as CSV.
//!
//! ```bash
//! cargo run --release --features ionex --example tec_maps > tec_statistics.csv
//! ```
#[path = "common/mod.rs"]
mod common;

use gnss_qc::prelude::*;

use ionex::prelude::{Key, Record, IONEX, TEC};

fn main() {
    let path = common::data_path("IONEX/V1/CKMG0080.09I.gz");
    let original = IONEX::from_gzip_file(&path).unwrap();

    let mut products = QcInputProducts::default();
    products
        .load_ionex(&QcInputConfig::default(), &path, original.clone())
        .unwrap();

    let cfg = QcTecConfig::default()
        .with_upsampling_factor(2.0)
        .with_interpolation(QcTecInterpolation::RotatedMaps)
        .with_points();

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_processor(QcTecProcessor::new(cfg))
        .build()
        .unwrap();

    // accumulates the nodes into a new IONEX record
    let mut record = Record::default();

    println!("epoch,source,region,interpolated,nodes,mean_tecu,std_tecu,min_tecu,max_tecu,max_latitude,max_longitude");

    for item in engine {
        let item = item.unwrap();

        match item.payload {
            QcOutput::TecPoint(node) => {
                let p = node.point;
                let mut tec = TEC::from_tecu(p.tecu);
                if let Some(rms) = p.rms_tecu {
                    tec = tec.with_rms(rms);
                }
                record.insert(
                    Key::from_decimal_degrees_km(
                        item.epoch,
                        p.latitude_deg,
                        p.longitude_deg,
                        p.altitude_km,
                    ),
                    tec,
                );
            }
            QcOutput::TecMap(s) => {
                println!(
                    "{},{},{},{},{},{:.2},{:.2},{:.1},{:.1},{:.1},{:.1}",
                    item.epoch,
                    s.source,
                    s.region,
                    s.interpolated,
                    s.nodes,
                    s.mean_tecu,
                    s.std_tecu,
                    s.min_tecu,
                    s.max_tecu,
                    s.max_latitude_deg,
                    s.max_longitude_deg
                );
            }
            _ => {}
        }
    }

    // new IONEX: same header, new temporal axis
    let mut synthesized = original.with_record(record);
    let epochs = synthesized.epoch_iter().collect::<Vec<_>>();

    synthesized.header.epoch_of_first_map = epochs[0];
    synthesized.header.epoch_of_last_map = *epochs.last().unwrap();
    synthesized.header.number_of_maps = epochs.len() as u32;
    synthesized.header.sampling_period = epochs[1] - epochs[0];
    synthesized
        .header
        .comments
        .push("Maps interpolated in time (x2, rotated maps) by GNSS-Qc".to_string());

    let output = std::env::temp_dir().join("CKMG0080_X2.09I");
    synthesized.to_file(&output).unwrap();

    eprintln!(
        "{} maps -> {} maps: {}",
        original.header.number_of_maps,
        synthesized.header.number_of_maps,
        output.display()
    );

    // the synthesized file is a valid IONEX, that reproduces the native maps
    let parsed = IONEX::from_file(&output).unwrap();
    assert_eq!(parsed.epoch_iter().count(), 25);
}
