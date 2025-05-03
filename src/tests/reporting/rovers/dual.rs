use maud::Render;
use std::fs::File;
use std::io::Write;

use crate::{
    prelude::{Epoch, QcConfig, QcContext, QcPreferedIndexing},
    tests::init_logger,
};

#[test]
fn html_summary_report() {
    init_logger();
    let mut ctx = QcContext::new();

    let now = Epoch::now().unwrap_or_else(|e| panic!("Failed to determine system time: {}", e));

    ctx.load_rinex_file("data/OBS/V3/DUTH0630.22O").unwrap();
    ctx.load_rinex_file("data/OBS/V3/LARM0010.22O").unwrap();
    ctx.load_rinex_file("data/OBS/V3/LARM0630.22O").unwrap();

    let report = ctx.summary_report(now);

    let rendered = report.render().into_string();

    let mut fd = File::create("index.html")
        .unwrap_or_else(|e| panic!("Failed to create index.html test file: {}", e));

    write!(fd, "{}", rendered).unwrap_or_else(|e| panic!("Failed to dump HTML: {}", e));
}

#[test]
fn html_observations_report() {
    init_logger();
    let mut ctx = QcContext::new();

    let now = Epoch::now().unwrap_or_else(|e| panic!("Failed to determine system time: {}", e));

    ctx.load_rinex_file("data/OBS/V3/DUTH0630.22O").unwrap();
    ctx.load_rinex_file("data/OBS/V3/LARM0010.22O").unwrap();
    ctx.load_rinex_file("data/OBS/V3/LARM0630.22O").unwrap();

    let indexes = ctx.observations.keys().collect::<Vec<_>>();
    assert_eq!(indexes.len(), 2);

    let report = ctx.report(now);

    let rendered = report.render().into_string();

    let mut fd = File::create("index.html")
        .unwrap_or_else(|e| panic!("Failed to create index.html test file: {}", e));

    write!(fd, "{}", rendered).unwrap_or_else(|e| panic!("Failed to dump HTML: {}", e));
}

#[test]
fn html_observations_gnss_rx_indexed_report() {
    init_logger();

    let cfg = QcConfig::default().with_prefered_indexing(QcPreferedIndexing::GnssReceiver);

    let mut ctx = QcContext::new().with_configuration_preferences(cfg);

    let now = Epoch::now().unwrap_or_else(|e| panic!("Failed to determine system time: {}", e));

    ctx.load_rinex_file("data/OBS/V3/DUTH0630.22O").unwrap();
    ctx.load_rinex_file("data/OBS/V3/LARM0010.22O").unwrap();
    ctx.load_rinex_file("data/OBS/V3/LARM0630.22O").unwrap();

    let indexes = ctx.observations.keys().collect::<Vec<_>>();
    assert_eq!(indexes.len(), 2);

    let report = ctx.report(now);

    let rendered = report.render().into_string();

    let mut fd = File::create("index.html")
        .unwrap_or_else(|e| panic!("Failed to create index.html test file: {}", e));

    write!(fd, "{}", rendered).unwrap_or_else(|e| panic!("Failed to dump HTML: {}", e));
}
