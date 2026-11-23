//! Helpers shared by the examples
#![allow(dead_code)]

use std::path::Path;

use gnss_qc::prelude::{QcInputConfig, QcInputProducts};

/// Loads files of the `data` submodule (RINEX, CRINEX, SP3, gzip compressed or not)
/// into a [QcInputProducts] table.
pub fn load_products(cfg: &QcInputConfig, files: &[&str]) -> QcInputProducts {
    let mut products = QcInputProducts::default();

    for file in files {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join(file);

        let result = if file.ends_with(".gz") {
            products.load_gzip_file(cfg, &path)
        } else {
            products.load_file(cfg, &path)
        };

        result.unwrap_or_else(|e| panic!("failed to load {}: {}", path.display(), e));
    }

    products
}

/// ANISE almanac, required by the navigation solver (Earth frame, Sun for eclipses).
/// Local files are used when `$GNSS_QC_ANISE_DIR` is defined (de440s.bsp or any JPL
/// planetary SPK like de421.bsp, and pck11.pca), otherwise ANISE downloads its
/// latest dataset (Internet access required, cached afterwards).
#[cfg(feature = "navigation")]
pub fn almanac() -> gnss_qc::prelude::Almanac {
    use gnss_qc::prelude::{Almanac, MetaAlmanac};

    match std::env::var("GNSS_QC_ANISE_DIR") {
        Ok(dir) => {
            let dir = Path::new(&dir);
            let spk = ["de440s.bsp", "de421.bsp"]
                .iter()
                .map(|name| dir.join(name))
                .find(|path| path.is_file())
                .expect("no planetary SPK (de440s.bsp) in $GNSS_QC_ANISE_DIR");

            Almanac::new(&spk.to_string_lossy())
                .and_then(|almanac| almanac.load(&dir.join("pck11.pca").to_string_lossy()))
                .expect("failed to load ANISE data")
        }
        Err(_) => MetaAlmanac::latest().expect("failed to download ANISE data"),
    }
}

/// Prints the solutions stream header (CSV)
#[cfg(feature = "navigation")]
pub fn print_header() {
    println!("epoch,type,x_m,y_m,z_m,lat_deg,long_deg,alt_m,clock_offset_s,gdop,satellites");
}

/// Prints one solution (CSV)
#[cfg(feature = "navigation")]
pub fn print_solution(solution: &gnss_qc::prelude::QcPvtSolution) {
    let pvt = &solution.solution;
    let (x, y, z) = pvt.pos_m;
    let (lat, long, alt) = pvt.lat_long_alt_deg_deg_m;

    println!(
        "{},{:?},{:.3},{:.3},{:.3},{:.8},{:.8},{:.3},{:.3E},{:.2},{}",
        pvt.epoch,
        pvt.solution_type,
        x,
        y,
        z,
        lat,
        long,
        alt,
        pvt.clock_offset_s,
        pvt.gdop,
        pvt.sv.len()
    );
}

/// 3D distance (m)
pub fn distance_m(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt()
}
