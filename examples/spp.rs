//! Single Point Positioning (SPP): single frequency code navigation.
//!
//! Station ESBC00DNK (Denmark), 2020-06-25, GPS + Galileo.
//! Run from the repository root (the data submodule is required):
//!
//! ```bash
//! cargo run --release --features navigation --example spp > solutions.csv
//! ```
#[path = "common/mod.rs"]
mod common;

use std::str::FromStr;

use gnss_qc::{geodesy::etrf2000_to_itrf2014, io::rinex::QcRinexObsSource, prelude::*};

const ROVER_OBS: &str = "CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz";

const NAV: &str = "NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz";
const SP3: &str = "SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz";

fn main() {
    // 1. Context: input products, indexed and merged by the products table.
    //    Observations are indexed by receiver (model + serial number),
    //    navigation and precise products by agency.
    let products = common::load_products(&QcInputConfig::default(), &[ROVER_OBS, NAV, SP3]);

    let rover = QcIndexing::Receiver("SEPT POLARX5 #3047937".to_string());

    // reference position of the rover antenna (RINEX header, ETRF2000)
    let reference_ecef_m = products
        .obs_rinex_iter()
        .find(|(indexing, _)| **indexing == rover)
        .and_then(|(_, rinex)| QcRinexObsSource::antenna_reference_point(rinex))
        .expect("rover reference position");

    // 2. Navigation processor
    let cfg = QcNavigationConfig::default()
        .with_method(Method::SPP)
        .with_constellations(&[Constellation::GPS, Constellation::Galileo])
        .with_orbits(QcOrbitPreference::Auto) // SP3 first, NAV otherwise
        .with_user_parameters(UserParameters::new(
            UserProfile::Static,
            ClockProfile::Quartz,
        ))
        .with_rover(rover.clone());

    let navigation = QcNavigationProcessor::new(common::almanac(), cfg)
        .expect("failed to deploy the navigation solver");

    // 3. MISO engine: all sources in, one stream out
    let t0 = Epoch::from_str("2020-06-25T00:00:00 GPST").unwrap();

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_time_window(Some(t0), Some(t0 + Duration::from_hours(1.0)))
        .with_processor(navigation)
        .build()
        .expect("invalid context");

    // 4. Consume the output stream
    common::print_header();

    let mut last = None;

    for item in engine {
        match item.expect("processing error").payload {
            QcOutput::PVTSolution(solution) => {
                common::print_solution(&solution);
                last = Some(solution);
            }
            QcOutput::NavigationReport(report) => eprintln!("{:#?}", report),
            _ => {}
        }
    }

    if let Some(last) = last {
        // absolute navigation with IGS products: ITRF2014
        let reference = etrf2000_to_itrf2014(reference_ecef_m, 2020.48);
        eprintln!(
            "last solution: {:.3} m away from the reference position",
            common::distance_m(last.solution.pos_m, reference)
        );
    }
}
