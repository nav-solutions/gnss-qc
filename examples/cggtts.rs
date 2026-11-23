//! CGGTTS solutions (common view time transfer): absolute navigation (CPP, GPS),
//! post-fitted over the BIPM tracking schedule, formatted into a CGGTTS file.
//!
//! Station ESBC00DNK (Denmark), 2020-06-25 00:00-03:00.
//! Run from the repository root (the data submodule is required):
//!
//! ```bash
//! cargo run --release --features cggtts --example cggtts > ESBC.cggtts
//! ```
#[path = "common/mod.rs"]
mod common;

use std::{io::BufWriter, str::FromStr};

use cggtts::prelude::Coordinates;
use gnss_qc::{io::rinex::QcRinexObsSource, prelude::*};

const OBS: &str = "CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz";
const NAV: &str = "NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz";
const SP3: &str = "SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz";

fn main() {
    // 1. Context
    let products = common::load_products(&QcInputConfig::default(), &[OBS, NAV, SP3]);

    // antenna position, written in the CGGTTS header
    let (x, y, z) = products
        .obs_rinex_iter()
        .find_map(|(_, rinex)| QcRinexObsSource::antenna_reference_point(rinex))
        .expect("antenna position");

    // 2. Navigation (absolute: SPP, CPP or PPP) + CGGTTS post-fit
    let navigation = QcNavigationConfig::default()
        .with_method(Method::CPP)
        .with_constellations(&[Constellation::GPS]) // one CGGTTS file per constellation
        .with_user_parameters(UserParameters::new(
            UserProfile::Static,
            ClockProfile::Quartz,
        ));

    let cggtts = QcCggttsProcessor::new(common::almanac(), navigation, QcCggttsConfig::default())
        .expect("failed to deploy the CGGTTS solver");

    // 3. Engine
    let t0 = Epoch::from_str("2020-06-25T00:00:00 GPST").unwrap();

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_time_window(Some(t0), Some(t0 + Duration::from_hours(3.0)))
        .with_processor(cggtts)
        .build()
        .expect("invalid context");

    // 4. Tracks stream: collect them into a CGGTTS file
    let mut file = CGGTTS::default();

    file.header = file
        .header
        .with_station("ESBC")
        .with_reference_frame("ITRF")
        .with_apc_coordinates(Coordinates { x, y, z })
        .with_comment("gnss-qc (CPP, GPS, GRG precise products)");

    for item in engine {
        match item.expect("processing error").payload {
            QcOutput::CggttsTrack(track) => file.tracks.push(*track.track),
            QcOutput::CggttsReport(report) => eprintln!("{:#?}", report),
            QcOutput::NavigationReport(report) => eprintln!("{:#?}", report),
            _ => {}
        }
    }

    let mut stdout = BufWriter::new(std::io::stdout());
    file.format(&mut stdout).expect("CGGTTS formatting");
}
