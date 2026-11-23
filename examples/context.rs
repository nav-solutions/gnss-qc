//! Context definition: loading input products into an indexed table.
//!
//! - Observation RINEX are indexed by receiver (model + serial number): the two stations
//!   (same receiver model) remain two distinct signal sources.
//! - Navigation RINEX are indexed by agency (or by a custom alias, see below).
//! - Files sharing the same indexing are merged (here: 48h of ephemerides).
//!
//! ```bash
//! cargo run --release --example context
//! ```
#[path = "common/mod.rs"]
mod common;

use gnss_qc::prelude::*;

fn main() {
    // default indexing
    let products = common::load_products(
        &QcInputConfig::default(),
        &[
            "CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz",
            "CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz",
            "NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz",
            "SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz",
        ],
    );

    println!("Observations (signal sources):");
    for (indexing, rinex) in products.obs_rinex_iter() {
        println!(
            "  {} - {} epochs, reference position: {:?}",
            indexing,
            rinex.record.as_obs().map(|r| r.len()).unwrap_or_default(),
            rinex.header.rx_position,
        );
    }

    println!("Navigation (ephemerides sources):");
    for (indexing, rinex) in products.nav_rinex_iter() {
        println!(
            "  {} - {} frames",
            indexing,
            rinex.record.as_nav().map(|r| r.len()).unwrap_or_default()
        );
    }

    println!("Precise products:");
    for (indexing, sp3) in products.sp3_iter() {
        println!("  {} - {} states", indexing, sp3.data.len());
    }

    // NYA1 NAV files do not describe their agency: they would be indexed by file name
    // and not merged. A custom alias gathers them in a single 48h ephemerides source.
    let cfg = QcInputConfig {
        nav_rinex_indexing: Some(QcIndexingConfig::Custom("NYA1-BRDC".to_string())),
        ..Default::default()
    };

    let products = common::load_products(
        &cfg,
        &[
            "NAV/V3/NYA100NOR_S_20241270000_01D_GN.rnx.gz",
            "NAV/V3/NYA100NOR_S_20241270000_01D_EN.rnx.gz",
            "NAV/V3/NYA100NOR_S_20241280000_01D_GN.rnx.gz",
            "NAV/V3/NYA100NOR_S_20241280000_01D_EN.rnx.gz",
        ],
    );

    println!("Custom indexing:");
    for (indexing, rinex) in products.nav_rinex_iter() {
        let record = rinex.record.as_nav().unwrap();
        println!(
            "  {} - {} frames from {} to {}",
            indexing,
            record.len(),
            record.keys().next().unwrap().epoch,
            record.keys().last().unwrap().epoch,
        );
    }

    // the table is then converted into engine sources
    let QcProductSources { sources, antex } = products.into_sources();
    println!(
        "{} engine source(s), {} ANTEX database(s)",
        sources.len(),
        antex.len()
    );
}
