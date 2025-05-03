use maud::Render;
use std::fs::File;
use std::io::Write;

use crate::{
    prelude::{Epoch, QcContext},
    tests::init_logger,
};

#[test]
fn html_summary_report() {
    init_logger();
    let mut ctx = QcContext::new();

    let now = Epoch::now().unwrap_or_else(|e| panic!("Failed to determine system time: {}", e));

    ctx.load_gzip_rinex_file("data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz")
        .unwrap();

    ctx.load_gzip_rinex_file("data/CLK/V3/GRG0MGXFIN_20201770000_01D_30S_CLK.CLK.gz")
        .unwrap();

    let report = ctx.summary_report(now);

    let rendered = report.render().into_string();

    let mut fd = File::create("index.html")
        .unwrap_or_else(|e| panic!("Failed to create index.html test file: {}", e));

    write!(fd, "{}", rendered).unwrap_or_else(|e| panic!("Failed to dump HTML: {}", e));
}

#[test]
fn html_report() {
    init_logger();
    let mut ctx = QcContext::new();

    let now = Epoch::now().unwrap_or_else(|e| panic!("Failed to determine system time: {}", e));

    ctx.load_gzip_rinex_file("data/NAV/V3/MOJN00DNK_R_20201770000_01D_MN.rnx.gz")
        .unwrap();

    ctx.load_gzip_rinex_file("data/CLK/V3/GRG0MGXFIN_20201770000_01D_30S_CLK.CLK.gz")
        .unwrap();

    assert!(ctx.precise_clock_data().is_some());
    assert!(ctx.brdc_navigation.is_some());

    let report = ctx.report(now);

    let rendered = report.render().into_string();

    let mut fd = File::create("index.html")
        .unwrap_or_else(|e| panic!("Failed to create index.html test file: {}", e));

    write!(fd, "{}", rendered).unwrap_or_else(|e| panic!("Failed to dump HTML: {}", e));
}
