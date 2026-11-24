//! Serde: every configuration, data point and result can be serialized and deserialized
//! (JSON round trip). The few types that can only be serialized are listed in
//! `serialize_only`, with the reason.
use std::fmt::Debug;

use serde::{de::DeserializeOwned, Serialize};

use rinex::prelude::obs::ClockObservation;

use crate::{
    io::input::types::{QcProductType, QcRINEXType},
    orbit::QcSatelliteState,
    prelude::*,
    tests::{
        miso::{load_products, load_rinex},
        toolkit::{ephemeris_item, epoch, gps_ephemeris, pseudo_range_item},
    },
};

const ESBC_OBS: &str = "CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz";
const MOJN_OBS: &str = "CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz";
const ESBC_NAV: &str = "NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz";

/// Serialize + Deserialize, at compile time
fn assert_serde<T: Serialize + DeserializeOwned>() {}

/// Serialize only, at compile time
fn assert_serialize<T: Serialize>() {}

/// JSON round trip, compared by value
fn round_trip<T: Serialize + DeserializeOwned + PartialEq + Debug>(value: &T) {
    crate::tests::serde_round_trip(value)
}

/// JSON round trip, for types without [PartialEq]: compared by serialized form
#[allow(dead_code)]
fn json_round_trip<T: Serialize + DeserializeOwned>(value: &T) {
    let json = serde_json::to_value(value).unwrap();
    let parsed: T = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(serde_json::to_value(&parsed).unwrap(), json);
}

#[test]
fn serde_traits() {
    // input
    assert_serde::<QcIndexing>();
    assert_serde::<QcIndexingConfig>();
    assert_serde::<QcInputConfig>();
    assert_serde::<crate::cfg::QcConfig>();
    assert_serde::<QcRINEXType>();
    assert_serde::<QcProductType>();

    // sources and engine
    assert_serde::<QcPoll<QcInputItem>>();
    assert_serde::<QcInputItem>();
    assert_serde::<QcPayload>();
    assert_serde::<QcSignal>();
    assert_serde::<QcPreciseState>();
    assert_serde::<QcClockId>();
    assert_serde::<QcPreciseClock>();
    assert_serde::<QcMeteoSample>();
    assert_serde::<QcTecPoint>();
    assert_serde::<QcTecValue>();
    assert_serde::<QcSourceDescriptor>();
    assert_serde::<QcEngineStats>();
    assert_serde::<QcInterpolatedState>();

    // orbits
    assert_serde::<QcOrbitPreference>();
    assert_serde::<QcOrbitKind>();
    assert_serde::<QcClockPreference>();
    assert_serde::<QcClockKind>();
    assert_serde::<QcSatelliteState>();
    assert_serde::<crate::orbit::QcKepler>();
    assert_serde::<crate::orbit::QcKeplerState>();

    // processors configurations
    assert_serde::<QcObservationResidualConfig>();
    assert_serde::<QcOrbitResidualConfig>();
    assert_serde::<QcOrbitProduct>();
    assert_serde::<QcPreciseClockConfig>();
    assert_serde::<QcClockProduct>();
    assert_serde::<QcClockAlignment>();
    assert_serde::<QcClockResidualConfig>();
    assert_serde::<QcTecConfig>();
    assert_serde::<QcTecRegion>();
    assert_serde::<QcTecSampling>();
    assert_serde::<QcTecInterpolation>();
    assert_serde::<QcMeteoConfig>();

    #[cfg(feature = "ionex")]
    assert_serde::<QcIppConfig>();

    #[cfg(feature = "navigation")]
    {
        assert_serde::<QcNavigationConfig>();
        assert_serde::<QcNavigationTechnique>();
    }

    // results
    assert_serde::<QcOutput>();
    assert_serde::<QcOutputItem<QcOutput>>();
    assert_serde::<QcSignalOutput>();
    assert_serde::<QcEphemerisCoverage>();
    assert_serde::<QcObservationResidual>();
    assert_serde::<QcOrbitResidual>();
    assert_serde::<QcPreciseClockOutput>();
    assert_serde::<QcClockResidual>();
    assert_serde::<QcTecMapPoint>();
    assert_serde::<QcTecMapStatistics>();
    assert_serde::<QcMeteoOutput>();

    #[cfg(feature = "navigation")]
    assert_serde::<QcNavigationReport>();

    #[cfg(feature = "ionex")]
    assert_serde::<QcIppOutput>();

    #[cfg(feature = "cggtts")]
    {
        assert_serde::<QcCggttsTrack>();
        assert_serde::<QcCggttsReport>();
    }
}

/// Types that can only be serialized, because they contain types of other libraries
/// that only implement Serialize:
/// - rinex `Ephemeris` (radio ephemerides);
/// - gnss-rtk `PVTSolution` (P.V.T solutions);
/// - cggtts `CommonViewCalendar` (no serde at all: QcCggttsConfig is not serializable).
///
/// The corresponding variants of [QcOutput] and [QcPayload] are serialized, but
/// rejected when deserializing.
#[test]
fn serialize_only() {
    assert_serialize::<QcEphemeris>();
    assert_serialize::<QcEphemerisOutput>();

    #[cfg(feature = "navigation")]
    assert_serialize::<QcPvtSolution>();

    // ephemeris payload: serialized, not deserialized
    let item = ephemeris_item("G01", epoch("2020-06-25T00:00:00 GPST"));
    let json = serde_json::to_string(&item).unwrap();
    assert!(json.contains("Ephemeris"));
    assert!(serde_json::from_str::<QcInputItem>(&json).is_err());

    let output = QcOutput::Ephemeris(QcEphemerisOutput {
        source: QcIndexing::Agency("IGS".to_string()),
        sv: SV::new(Constellation::GPS, 1),
        toc: epoch("2020-06-25T00:00:00 GPST"),
        ephemeris: gps_ephemeris(
            SV::new(Constellation::GPS, 1),
            epoch("2020-06-25T00:00:00 GPST"),
        )
        .ephemeris,
    });

    let json = serde_json::to_string(&output).unwrap();
    assert!(serde_json::from_str::<QcOutput>(&json).is_err());
}

#[test]
fn input_configuration_serde() {
    for indexing in [
        QcIndexing::Antenna("TRM59800.00 SCIS".to_string()),
        QcIndexing::Receiver("SEPT POLARX5 #3047937".to_string()),
        QcIndexing::Operator("SDFE".to_string()),
        QcIndexing::Agency("GRGS".to_string()),
        QcIndexing::Custom("agency:GRGS".to_string()),
    ] {
        round_trip(&indexing);
    }

    for preference in [
        QcIndexingConfig::Antenna,
        QcIndexingConfig::Receiver,
        QcIndexingConfig::Operator,
        QcIndexingConfig::Agency,
        QcIndexingConfig::Custom("NYA1-BRDC".to_string()),
        QcIndexingConfig::FileName,
    ] {
        round_trip(&preference);
    }

    round_trip(&QcInputConfig::default());

    let cfg = QcInputConfig {
        obs_rinex_indexing: Some(QcIndexingConfig::Antenna),
        nav_rinex_indexing: Some(QcIndexingConfig::Custom("BRDC".to_string())),
        meteo_rinex_indexing: None,
        clk_rinex_indexing: Some(QcIndexingConfig::FileName),
        sp3_indexing: Some(QcIndexingConfig::FileName),
        ionex_indexing: Some(QcIndexingConfig::Custom("GIM".to_string())),
    };

    round_trip(&cfg);
    round_trip(&crate::cfg::QcConfig { input: cfg });

    // missing fields take their default value
    let cfg: QcInputConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(cfg, QcInputConfig::default());

    let cfg: QcInputConfig = serde_json::from_str(r#"{"sp3_indexing":"FileName"}"#).unwrap();
    assert_eq!(cfg.sp3_indexing, Some(QcIndexingConfig::FileName));
    assert_eq!(cfg.obs_rinex_indexing, Some(QcIndexingConfig::Receiver));

    for t in [
        QcRINEXType::Observation,
        QcRINEXType::Meteo,
        QcRINEXType::Navigation,
        QcRINEXType::Clock,
        QcRINEXType::Antenna,
    ] {
        round_trip(&t);
        round_trip(&QcProductType::RINEX(t));
    }

    #[cfg(feature = "sp3")]
    round_trip(&QcProductType::SP3);
}

#[test]
fn processors_configuration_serde() {
    round_trip(&QcOrbitPreference::PreciseOnly);
    round_trip(&QcOrbitKind::Broadcast);

    for clocks in [
        QcClockPreference::Auto,
        QcClockPreference::Precise,
        QcClockPreference::Broadcast,
    ] {
        round_trip(&clocks);
    }

    for kind in [
        QcClockKind::PreciseClock,
        QcClockKind::PreciseState,
        QcClockKind::Broadcast,
    ] {
        round_trip(&kind);
    }

    round_trip(&QcPreciseClockConfig::default());
    round_trip(&QcPreciseClockConfig::stations_only().with_sampling(Duration::from_seconds(300.0)));
    round_trip(&QcMeteoConfig::default());

    round_trip(&QcClockResidualConfig::default());
    round_trip(
        &QcClockResidualConfig::clock_products(
            QcIndexing::Agency("GRG".to_string()),
            QcIndexing::Agency("JPL".to_string()),
        )
        .with_sampling(Duration::from_seconds(300.0))
        .with_alignment(QcClockAlignment::Epoch)
        .with_constellations(&[Constellation::GPS])
        .stations_only(),
    );
    round_trip(&QcClockProduct::Broadcast(None));

    round_trip(&QcTecConfig::default());
    round_trip(
        &QcTecConfig::default()
            .with_upsampling_factor(2.0)
            .with_interpolation(QcTecInterpolation::Linear)
            .with_points()
            .with_regions(vec![QcTecRegion::new(
                "Europe",
                (35.0, 70.0),
                (-10.0, 30.0),
            )]),
    );
    round_trip(&QcTecSampling::Period(Duration::from_seconds(1800.0)));
    round_trip(&QcMeteoConfig::default().with_observables(&[
        rinex::prelude::Observable::Pressure,
        rinex::prelude::Observable::Temperature,
    ]));

    round_trip(&QcObservationResidualConfig::default());
    round_trip(
        &QcObservationResidualConfig::new(
            QcIndexing::Receiver("A".to_string()),
            QcIndexing::Receiver("B".to_string()),
        )
        .with_all_observables(),
    );

    let cfg = QcObservationResidualConfig {
        tolerance: Some(Duration::from_milliseconds(500.0)),
        ..Default::default()
    };

    round_trip(&cfg);

    round_trip(&QcOrbitResidualConfig::default());
    round_trip(
        &QcOrbitResidualConfig::precise(
            QcIndexing::Agency("GRGS".to_string()),
            QcIndexing::Agency("IAC".to_string()),
        )
        .with_sampling(Duration::from_seconds(900.0))
        .with_constellations(&[Constellation::GPS, Constellation::Galileo])
        .with_max_broadcast_age(Duration::from_hours(1.0)),
    );

    round_trip(&QcOrbitProduct::Broadcast(None));
    round_trip(&QcOrbitProduct::Precise(Some(QcIndexing::Agency(
        "COD".to_string(),
    ))));

    #[cfg(feature = "ionex")]
    {
        round_trip(&QcIppConfig::default());
        round_trip(&QcIppConfig {
            orbits: QcOrbitPreference::BroadcastOnly,
            receivers: Some(vec![QcIndexing::Receiver("A".to_string())]),
            constellations: Some(vec![Constellation::BeiDou]),
            receiver_position_ecef_m: Some((3_582_105.0, 532_589.0, 5_232_754.0)),
            ..Default::default()
        });
    }

    #[cfg(feature = "navigation")]
    {
        round_trip(&QcNavigationTechnique::Absolute);
        round_trip(&QcNavigationTechnique::Rtk {
            base: QcIndexing::Receiver("BASE".to_string()),
            position_ecef_m: Some((1.0, 2.0, 3.0)),
        });

        json_round_trip(&QcNavigationConfig::default());
        json_round_trip(&QcNavigationConfig::default().with_clocks(QcClockPreference::Precise));

        // configurations written before the clocks preference
        let mut json = serde_json::to_value(QcNavigationConfig::default()).unwrap();
        json.as_object_mut().unwrap().remove("clocks");
        let cfg: QcNavigationConfig = serde_json::from_value(json).unwrap();
        assert_eq!(cfg.clocks, QcClockPreference::Auto);
        json_round_trip(
            &QcNavigationConfig::default()
                .with_method(Method::PPP)
                .with_orbits(QcOrbitPreference::PreciseOnly)
                .with_rtk_base(QcIndexing::Receiver("BASE".to_string())),
        );
    }
}

#[test]
fn data_points_serde() {
    let t = epoch("2020-06-25T00:00:30 GPST");
    let sv = SV::new(Constellation::GPS, 1);

    // precise clocks: satellites and stations
    round_trip(&QcInputItem::precise_clock(
        t,
        QcClockId::Satellite(sv),
        rinex::clock::ClockProfileType::AS,
        1.59438015248E-05,
    ));

    let mut station = QcInputItem::precise_clock(
        t,
        QcClockId::Station("ESBC00DNK".to_string()),
        rinex::clock::ClockProfileType::AR,
        -1.0E-9,
    );

    if let QcPayload::PreciseClock(clock) = &mut station.payload {
        clock.offset_dev_s = Some(1.0E-11);
        clock.drift_s_s = Some(1.0E-13);
    }

    round_trip(&station);

    round_trip(&QcInputItem::meteo(
        t,
        rinex::prelude::Observable::Pressure,
        1013.25,
    ));

    // descriptors written before the precise clocks and meteo sources
    let descriptor = QcSourceDescriptor::new(QcIndexing::Agency("GRG".to_string()))
        .with_precise_clocks()
        .with_meteo();
    round_trip(&descriptor);

    let mut json = serde_json::to_value(&descriptor).unwrap();
    json.as_object_mut().unwrap().remove("precise_clocks");
    json.as_object_mut().unwrap().remove("meteo");
    let parsed: QcSourceDescriptor = serde_json::from_value(json).unwrap();
    assert!(!parsed.precise_clocks && !parsed.meteo);

    let signal = pseudo_range_item("G01", t, 20_000_123.456);
    round_trip(&signal);

    let precise = QcInputItem::precise_state(t, sv, (15_000.1, -20_000.2, 5_000.3), Some(1.0E-4));
    round_trip(&precise);
    round_trip(&QcInputItem::precise_state(t, sv, (1.0, 2.0, 3.0), None));

    let clock =
        QcInputItem::receiver_clock(t, ClockObservation::default().with_offset_s(t, 1.0E-3));
    round_trip(&clock);

    round_trip(&QcPoll::Ready(signal));
    round_trip(&QcPoll::<QcInputItem>::Pending);
    round_trip(&QcPoll::<QcInputItem>::EndOfStream);

    round_trip(
        &QcSourceDescriptor::new(QcIndexing::Receiver("A".to_string()))
            .with_signals()
            .with_reference_position_ecef_m((3_582_105.0, 532_589.0, 5_232_754.0)),
    );

    round_trip(&QcEngineStats {
        epochs: 10,
        items: 100,
        late_items: 1,
        unhandled_items: 2,
    });

    round_trip(&QcInterpolatedState {
        position_km: (1.0, 2.0, 3.0),
        velocity_km_s: (0.1, 0.2, 0.3),
        clock_offset_s: Some(1.0E-6),
    });

    round_trip(&QcSatelliteState {
        kind: QcOrbitKind::Precise,
        position_m: (1.0, 2.0, 3.0),
        velocity_m_s: (0.1, 0.2, 0.3),
    });
}

/// Keplerian parameters and states from a real navigation file
#[test]
fn kepler_serde() {
    use crate::orbit::QcKepler;

    let rinex = load_rinex(ESBC_NAV);

    let (sv, toc, eph) = rinex
        .record
        .as_nav()
        .unwrap()
        .iter()
        .find_map(|(k, frame)| match frame {
            rinex::navigation::NavFrame::EPH(eph) if k.sv.constellation == Constellation::GPS => {
                Some((k.sv, k.epoch, eph.clone()))
            }
            _ => None,
        })
        .unwrap();

    let kepler = QcKepler::from_ephemeris(sv, toc, &eph).unwrap();
    round_trip(&kepler);

    let state = kepler.state(toc + Duration::from_seconds(600.0));
    round_trip(&state);
}

/// The output stream of a real session can be recorded (JSON lines) and read back
#[test]
fn output_stream_serde() {
    let t0 = epoch("2020-06-25T00:00:00 GPST");

    let products = load_products(&QcInputConfig::default(), &[ESBC_OBS, MOJN_OBS, ESBC_NAV]);

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_time_window(Some(t0), Some(t0 + Duration::from_seconds(300.0)))
        .with_processor(QcSignalProcessor)
        .with_processor(QcEphemerisCoverageProcessor::default())
        .with_processor(QcObservationResidualProcessor::new(
            QcObservationResidualConfig::default(),
        ))
        .with_processor(QcOrbitResidualProcessor::new(
            QcOrbitResidualConfig::broadcast(
                QcIndexing::Custom("ESBC00DNK_R_20201770000_01D_MN".to_string()),
                QcIndexing::Custom("ESBC00DNK_R_20201770000_01D_MN".to_string()),
            ),
        ))
        .build()
        .unwrap();

    let mut counts = std::collections::HashMap::<&str, usize>::new();

    for item in engine {
        let item = item.unwrap();

        let kind = match &item.payload {
            QcOutput::Signal(_) => "signal",
            QcOutput::EphemerisCoverage(_) => "coverage",
            QcOutput::ObservationResidual(_) => "observation residual",
            _ => "other",
        };

        *counts.entry(kind).or_default() += 1;

        round_trip(&item);
    }

    assert!(counts["signal"] > 1000, "{:?}", counts);
    assert!(counts["coverage"] > 10, "{:?}", counts);
    assert!(counts["observation residual"] > 1000, "{:?}", counts);
}

/// Orbit residuals of real products
#[test]
#[cfg(feature = "sp3")]
fn orbit_residuals_serde() {
    let products = load_products(
        &QcInputConfig::default(),
        &[ESBC_NAV, "SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz"],
    );

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_processor(QcOrbitResidualProcessor::new(
            QcOrbitResidualConfig::broadcast_precise()
                .with_max_broadcast_age(Duration::from_hours(1.0)),
        ))
        .build()
        .unwrap();

    let mut n = 0;

    for item in engine.take(500) {
        let item = item.unwrap();
        assert!(matches!(item.payload, QcOutput::OrbitResidual(_)));
        round_trip(&item);
        n += 1;
    }

    assert_eq!(n, 500);
}

/// Precise clocks and meteo observations of real products
#[test]
fn clock_meteo_outputs_serde() {
    let products = load_products(
        &QcInputConfig::default(),
        &[
            "CLK/V3/COD0MGXFIN_20241280000_01D_30S_CLK.CLK.gz",
            "MET/V3/POTS00DEU_R_20232540000_01D_05M_MM.rnx.gz",
        ],
    );

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_processor(QcPreciseClockProcessor::new(
            QcPreciseClockConfig::default().with_sampling(Duration::from_hours(1.0)),
        ))
        .with_processor(QcMeteoProcessor::default())
        .build()
        .unwrap();

    let (mut clocks, mut meteo) = (0, 0);

    for item in engine {
        let item = item.unwrap();

        match &item.payload {
            QcOutput::PreciseClock(_) => clocks += 1,
            QcOutput::Meteo(_) => meteo += 1,
            _ => panic!("unexpected output"),
        }

        round_trip(&item);
    }

    assert!(clocks > 5000, "{}", clocks);
    assert_eq!(meteo, 864);
}

/// Clock residuals and TEC maps of real products
#[test]
#[cfg(feature = "ionex")]
fn clock_residuals_tec_outputs_serde() {
    let products = load_products(
        &QcInputConfig::default(),
        &[
            "CLK/V3/COD0MGXFIN_20241280000_01D_30S_CLK.CLK.gz",
            "IONEX/V1/CKMG0080.09I.gz",
        ],
    );

    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_processor(QcTecProcessor::new(
            QcTecConfig::default().with_upsampling_factor(2.0),
        ))
        .build()
        .unwrap();

    let mut tec = 0;

    for item in engine {
        let item = item.unwrap();
        if matches!(item.payload, QcOutput::TecMap(_)) {
            tec += 1;
        }
        round_trip(&item);
    }

    assert_eq!(tec, 25 * 6);

    let item = QcOutputItem {
        epoch: epoch("2020-06-25T00:00:00 GPST"),
        payload: QcOutput::ClockResidual(QcClockResidual {
            clock: QcClockId::Satellite(SV::new(Constellation::GPS, 1)),
            lhs_source: QcIndexing::Agency("GRG".to_string()),
            lhs_kind: QcClockKind::PreciseClock,
            rhs_source: QcIndexing::Agency("JPL".to_string()),
            rhs_kind: QcClockKind::PreciseClock,
            residual_s: 1.234E-9,
            datum_s: Some(1.1E-9),
        }),
    };
    round_trip(&item);

    let item = QcOutputItem {
        epoch: epoch("2009-01-08T01:00:00 UTC"),
        payload: QcOutput::TecPoint(QcTecMapPoint {
            source: QcIndexing::Agency("AIUB".to_string()),
            interpolated: true,
            point: QcTecPoint {
                latitude_deg: 43.5,
                longitude_deg: 1.5,
                altitude_km: 350.0,
                tecu: 12.3,
                rms_tecu: None,
            },
        }),
    };
    round_trip(&item);
}
