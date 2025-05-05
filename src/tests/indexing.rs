use crate::{
    config::QcConfig,
    context::{QcContext, QcIndexing},
    prelude::QcPreferedIndexing,
};

#[test]
fn default_observations_indexing() {
    let mut ctx = QcContext::new();

    ctx.load_rinex_file("data/CRNX/V1/AJAC3550.21D").unwrap();

    let geo_marker = QcIndexing::GeodeticMarker("AJAC-10077M005".to_string());
    let invalid_marker = QcIndexing::GeodeticMarker("Invalid".to_string());
    let gnss_rx = QcIndexing::GnssReceiver("LEICA GR50-209088".to_string());

    assert!(
        ctx.rinex_observation_data(&geo_marker).is_some(),
        "Geodetic marker is default indexer"
    );

    assert!(
        ctx.rinex_observation_data(&invalid_marker).is_none(),
        "non existing (invalid) marker"
    );

    assert!(
        ctx.rinex_observation_data(&gnss_rx).is_none(),
        "Geodetic marker should have been prefered"
    );
}

#[test]
fn prefered_gnss_receiver_indexing() {
    let cfg = QcConfig::default().with_prefered_indexing(QcPreferedIndexing::GnssReceiver);

    let mut ctx = QcContext::new().with_configuration_preferences(cfg);

    ctx.load_rinex_file("data/CRNX/V1/AJAC3550.21D").unwrap();

    let geo_marker = QcIndexing::GeodeticMarker("AJAC-10077M005".to_string());
    let gnss_rx = QcIndexing::GnssReceiver("LEICA GR50-2090088".to_string());

    assert!(
        ctx.rinex_observation_data(&geo_marker).is_none(),
        "GNSS-RX should have been prefered"
    );

    assert!(
        ctx.rinex_observation_data(&gnss_rx).is_some(),
        "GNSS-RX set as prefered indexer"
    );
}

#[test]
fn prefered_operator_indexing() {
    let cfg = QcConfig::default().with_prefered_indexing(QcPreferedIndexing::Operator);

    let mut ctx = QcContext::new().with_configuration_preferences(cfg);

    ctx.load_rinex_file("data/CRNX/V1/AJAC3550.21D").unwrap();

    let geo_marker = QcIndexing::GeodeticMarker("AJAC-10077M005".to_string());
    let gnss_rx = QcIndexing::GnssReceiver("LEICA GR50-2090088".to_string());
    let operator = QcIndexing::Operator("Automatic".to_string());

    assert!(
        ctx.rinex_observation_data(&geo_marker).is_none(),
        "Operator should have been prefered"
    );

    assert!(
        ctx.rinex_observation_data(&gnss_rx).is_none(),
        "Operator should have been prefered"
    );

    assert!(
        ctx.rinex_observation_data(&operator).is_some(),
        "Operator should have been prefered"
    );
}
