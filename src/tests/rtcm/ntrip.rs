//! NTRIP clients, against local casters (mock servers) that stream real data
//! (RINEX files encoded as RTCM 3 frames).
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::Duration as StdDuration,
};

use super::*;

use crate::tests::miso::{EpochRecorder, TestOutput};

pub const MOJN_OBS: &str = "CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz";

/// "user:secret", base 64
const CREDENTIALS: &str = "dXNlcjpzZWNyZXQ=";

/// Local NTRIP caster, serving one mount point to one client, then closing the connection.
/// Returns the port of the caster.
pub fn caster(mount: &'static str, credentials: Option<&'static str>, frames: Vec<Vec<u8>>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();

        // HTTP request
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];

        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
            match socket.read(&mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(n) => request.extend_from_slice(&buffer[..n]),
            }
        }

        let request = String::from_utf8_lossy(&request).to_string();

        if !request.starts_with(&format!("GET /{} ", mount)) {
            let _ = socket.write_all(b"HTTP/1.0 404 Not Found\r\n\r\n");
            return;
        }

        if let Some(credentials) = credentials {
            // (header names are case insensitive)
            if !request.to_lowercase().contains(&format!(
                "authorization: basic {}",
                credentials.to_lowercase()
            )) {
                let _ = socket.write_all(b"HTTP/1.0 401 Unauthorized\r\n\r\n");
                return;
            }
        }

        socket.write_all(b"ICY 200 OK\r\n\r\n").unwrap();

        // real-time pace: bursts of frames
        for (index, frame) in frames.iter().enumerate() {
            if socket.write_all(frame).is_err() {
                return; // client left
            }
            if index % 32 == 0 {
                thread::sleep(StdDuration::from_millis(1));
            }
        }

        // end of stream: connection closed
    });

    port
}

#[test]
fn ntrip_urls() {
    let cfg = "ntrip://user:secret@caster.centipede.fr:2101/MOJN"
        .parse::<QcNtripConfig>()
        .unwrap();

    assert_eq!(cfg.host, "caster.centipede.fr");
    assert_eq!(cfg.port, 2101);
    assert!(!cfg.use_tls);
    assert_eq!(cfg.mount, "MOJN");
    assert_eq!(cfg.user.as_deref(), Some("user"));
    assert_eq!(cfg.password.as_deref(), Some("secret"));
    assert_eq!(cfg.role, QcNtripRole::Rover);
    assert_eq!(cfg.indexing(), QcIndexing::Custom("MOJN".to_string()));

    // the password is never displayed
    assert_eq!(
        cfg.to_string(),
        "ntrip://user@caster.centipede.fr:2101/MOJN"
    );
    assert!(!format!("{:?}", cfg).contains("secret"));

    let cfg = "ntrips://ntrip.data.gnss.ga.gov.au:443/ALIC00AUS0"
        .parse::<QcNtripConfig>()
        .unwrap();
    assert!(cfg.use_tls);
    assert_eq!(cfg.port, 443);
    assert_eq!(cfg.user, None);

    // default port, no scheme
    let cfg = "rtk2go.com/MOUNT".parse::<QcNtripConfig>().unwrap();
    assert_eq!(cfg.port, 2101);
    assert_eq!(cfg.mount, "MOUNT");

    for invalid in [
        "ntrip://caster:2101",
        "ntrip://caster:2101/",
        "ntrip://caster:port/MOUNT",
        "ftp://caster/MOUNT",
        "ntrip:///MOUNT",
    ] {
        assert!(invalid.parse::<QcNtripConfig>().is_err(), "{}", invalid);
    }

    for (role, expected) in [
        ("rover", QcNtripRole::Rover),
        ("base", QcNtripRole::Base),
        ("ephemerides", QcNtripRole::Ephemerides),
    ] {
        assert_eq!(role.parse::<QcNtripRole>().unwrap(), expected);
    }

    // roles and descriptors
    let cfg =
        QcNtripConfig::new("localhost", 2101, "BCEP00BKG0").with_role(QcNtripRole::Ephemerides);
    let descriptor = cfg.descriptor();
    assert!(
        !descriptor.signals,
        "ephemerides streams must never be waited for"
    );
    assert!(descriptor.ephemerides);

    let cfg = QcNtripConfig::new("localhost", 2101, "MOJN")
        .with_role(QcNtripRole::Base)
        .with_indexing(QcIndexing::Custom("BASE".to_string()))
        .with_reference_position_ecef_m((1.0, 2.0, 3.0));
    let descriptor = cfg.descriptor();
    assert!(descriptor.signals);
    assert_eq!(descriptor.indexing, QcIndexing::Custom("BASE".to_string()));
    assert_eq!(descriptor.reference_position_ecef_m, Some((1.0, 2.0, 3.0)));

    #[cfg(feature = "serde")]
    {
        let json = r#"{"host":"caster.centipede.fr","mount":"MOJN","role":"Base"}"#;
        let cfg: QcNtripConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.port, 2101);
        assert_eq!(cfg.role, QcNtripRole::Base);
    }
}

/// One NTRIP client, rover role: same epochs and satellites as the RINEX file,
/// ephemerides delivered by the stream.
#[test]
fn ntrip_rover() {
    let end = epoch("2020-06-25T01:00:00 GPST");

    let port = caster(
        "ESBC00DNK0",
        Some(CREDENTIALS),
        station_frames(ESBC_OBS, Some(ESBC_NAV), None, end),
    );

    let cfg = QcNtripConfig::new("127.0.0.1", port, "ESBC00DNK0")
        .with_credentials("user", "secret")
        .with_reference_epoch(reference_epoch());

    let client = QcNtripSource::new(cfg).unwrap();

    let engine = QcEngine::<TestOutput>::builder()
        .with_source(client)
        .with_processor(EpochRecorder::default())
        .build()
        .unwrap();

    let mut epochs = Vec::new();

    for item in engine {
        if let TestOutput::Epoch(summary) = item.unwrap().payload {
            epochs.push(summary);
        }
    }

    let reference = rinex_epochs(ESBC_OBS, end);
    assert_eq!(epochs.len(), reference.len());

    for (summary, (t, _)) in epochs.iter().zip(reference.iter()) {
        assert_eq!(summary.epoch, *t);
        assert_eq!(
            summary.receivers[0].receiver,
            QcIndexing::Custom("ESBC00DNK0".to_string())
        );
        assert!(summary.receivers[0].satellites.len() > 20);
        assert!(summary.buffered_ephemerides > 0);
    }
}

/// Connection failures are reported by the engine
#[test]
fn ntrip_failures() {
    let end = epoch("2020-06-25T00:01:00 GPST");

    // wrong credentials
    let port = caster("MOUNT", Some(CREDENTIALS), vec![]);
    let cfg = QcNtripConfig::new("127.0.0.1", port, "MOUNT").with_credentials("user", "wrong");

    // unknown mount point
    let port2 = caster("MOUNT", None, station_frames(ESBC_OBS, None, None, end));
    let cfg2 = QcNtripConfig::new("127.0.0.1", port2, "UNKNOWN");

    // nobody listening
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port3 = listener.local_addr().unwrap().port();
    drop(listener);
    let cfg3 = QcNtripConfig::new("127.0.0.1", port3, "MOUNT");

    for cfg in [cfg, cfg2, cfg3] {
        let mount = cfg.to_string();
        let client = QcNtripSource::new(cfg).unwrap();

        let engine = QcEngine::<TestOutput>::builder()
            .with_source(client)
            .with_processor(EpochRecorder::default())
            .build()
            .unwrap();

        let results = engine.collect::<Vec<_>>();

        assert_eq!(results.len(), 1, "{}", mount);
        let error = results[0].as_ref().unwrap_err().to_string();
        println!("{}: {}", mount, error);
        assert!(error.contains("NTRIP connection failure"), "{}", error);
    }
}

/// RTK-CPP: two NTRIP clients (rover ESBC00DNK, base MOJN00DNK, ~65 km), real-time.
/// The base stream also delivers the radio ephemerides and its coordinates (RTCM 1005).
/// Same solutions as the file based scenario (RINEX files).
#[cfg(feature = "navigation")]
#[test]
fn ntrip_rtk_cpp() {
    use crate::{
        io::rinex::QcRinexObsSource,
        tests::navigation::{almanac, esbc, mojn, scenarios::navigate},
    };

    let start = epoch("2020-06-25T00:00:00 GPST");
    let end = epoch("2020-06-25T01:00:00 GPST");

    let user = UserParameters::new(UserProfile::Static, ClockProfile::Quartz);

    let cfg = |rover: QcIndexing, base: QcIndexing| {
        QcNavigationConfig::default()
            .with_method(Method::CPP)
            .with_user_parameters(user)
            .with_rover(rover)
            .with_rtk_base(base)
    };

    // reference: files
    let products = crate::tests::miso::load_products(
        &QcInputConfig::default(),
        &[ESBC_OBS, MOJN_OBS, ESBC_NAV],
    );
    let reference = navigate(products, vec![], cfg(esbc(), mojn()), (start, end));

    // base position: RINEX header of MOJN, streamed as RTCM 1005
    let mojn_position = QcRinexObsSource::antenna_reference_point(&load_rinex(MOJN_OBS)).unwrap();

    let rover_port = caster(
        "ESBC00DNK0",
        None,
        station_frames(ESBC_OBS, None, None, end),
    );

    let base_port = caster(
        "MOJN00DNK0",
        Some(CREDENTIALS),
        station_frames(MOJN_OBS, Some(ESBC_NAV), Some(mojn_position), end),
    );

    let streams = [
        QcNtripConfig::new("127.0.0.1", rover_port, "ESBC00DNK0")
            .with_role(QcNtripRole::Rover)
            .with_reference_epoch(reference_epoch()),
        QcNtripConfig::new("127.0.0.1", base_port, "MOJN00DNK0")
            .with_role(QcNtripRole::Base)
            .with_credentials("user", "secret")
            .with_reference_epoch(reference_epoch()),
    ];

    let nav_cfg = QcNavigationConfig::default()
        .with_method(Method::CPP)
        .with_user_parameters(user)
        .with_ntrip_roles(&streams);

    assert_eq!(
        nav_cfg.rover,
        Some(QcIndexing::Custom("ESBC00DNK0".to_string()))
    );

    let mut builder = QcEngine::<crate::tests::navigation::scenarios::NavOutput>::builder()
        .with_time_window(Some(start), Some(end))
        .with_processor(QcNavigationProcessor::new(almanac(), nav_cfg).unwrap());

    for stream in streams.iter() {
        builder = builder.with_source(QcNtripSource::new(stream.clone()).unwrap());
    }

    let engine = builder.build().unwrap();

    let mut solutions = Vec::new();
    let mut report = None;

    for item in engine {
        match item.unwrap().payload {
            crate::tests::navigation::scenarios::NavOutput::Builtin(output) => match output {
                QcOutput::PVTSolution(solution) => solutions.push(solution),
                QcOutput::NavigationReport(r) => report = Some(r),
                _ => {}
            },
        }
    }

    let report = report.expect("navigation report");
    println!("{:?}", report);

    assert_eq!(
        report.solutions, reference.report.solutions,
        "{:?}",
        report.failures
    );
    assert_eq!(solutions.len(), reference.solutions.len());

    let mut max_m: f64 = 0.0;
    let mut sum_m2 = 0.0;

    for (streamed, file) in solutions.iter().zip(reference.solutions.iter()) {
        assert_eq!(streamed.solution.epoch, file.solution.epoch);
        assert_eq!(
            streamed.receiver,
            QcIndexing::Custom("ESBC00DNK0".to_string())
        );
        assert_eq!(streamed.solution.solution_type, PVTSolutionType::RTK);

        let (x, y, z) = streamed.solution.pos_m;
        let (rx, ry, rz) = file.solution.pos_m;
        let delta_m = ((x - rx).powi(2) + (y - ry).powi(2) + (z - rz).powi(2)).sqrt();
        max_m = max_m.max(delta_m);
        sum_m2 += delta_m * delta_m;
    }

    let rms_m = (sum_m2 / solutions.len() as f64).sqrt();

    println!(
        "difference with the file based solutions: max. {:.4} m, rms {:.4} m",
        max_m, rms_m
    );
    // RTCM resolution (pseudo ranges: 0.6 mm, ephemerides)
    assert!(
        max_m < 0.005,
        "streamed and file based solutions differ: {} m",
        max_m
    );
}

/// Real-time TEC of a network: two NTRIP clients (ESBC00DNK, MOJN00DNK), the coordinates
/// of both stations (RTCM 1005) and the radio ephemerides streamed by ESBC.
/// Same estimates as the file based scenario (RINEX files), see examples/ntrip_tec.
#[cfg(feature = "ionex")]
#[test]
fn ntrip_obs_tec() {
    use crate::io::rinex::QcRinexObsSource;
    use std::collections::HashMap;

    let end = epoch("2020-06-25T01:00:00 GPST");

    let cfg = || QcObsTecConfig::default().with_constellations(&[Constellation::GPS]);

    type Estimates = HashMap<(String, SV, Epoch), QcTecEstimate>;

    let collect = |engine: QcEngine<QcOutput>, station: &dyn Fn(&QcIndexing) -> String| {
        let (mut estimates, mut arcs) = (Estimates::new(), 0);
        for item in engine {
            let item = item.unwrap();
            match item.payload {
                QcOutput::TecEstimate(e) => {
                    if item.epoch <= end {
                        estimates.insert((station(&e.receiver), e.sv, item.epoch), *e);
                    }
                }
                QcOutput::TecArc(_) => arcs += 1,
                _ => {}
            }
        }
        (estimates, arcs)
    };

    // reference: files
    let products = crate::tests::miso::load_products(
        &QcInputConfig::default(),
        &[ESBC_OBS, MOJN_OBS, ESBC_NAV],
    );
    let engine = QcEngine::<QcOutput>::builder()
        .with_products(products)
        .with_time_window(None, Some(end))
        .with_processor(QcObsTecProcessor::new(cfg()))
        .build()
        .unwrap();

    let (reference, _) = collect(engine, &|rx| {
        if rx.to_string().contains("3047937") {
            "ESBC".to_string()
        } else {
            "MOJN".to_string()
        }
    });

    // streams: coordinates of each station (RINEX headers, as RTCM 1005)
    let position = |obs: &str| QcRinexObsSource::antenna_reference_point(&load_rinex(obs)).unwrap();

    let esbc_port = caster(
        "ESBC00DNK0",
        None,
        station_frames(ESBC_OBS, Some(ESBC_NAV), Some(position(ESBC_OBS)), end),
    );
    let mojn_port = caster(
        "MOJN00DNK0",
        None,
        station_frames(MOJN_OBS, None, Some(position(MOJN_OBS)), end),
    );

    let mut builder = QcEngine::<QcOutput>::builder()
        .with_time_window(None, Some(end))
        .with_processor(QcObsTecProcessor::new(cfg()));

    for (mount, port) in [("ESBC00DNK0", esbc_port), ("MOJN00DNK0", mojn_port)] {
        let stream = QcNtripConfig::new("127.0.0.1", port, mount)
            .with_role(QcNtripRole::Base)
            .with_reference_epoch(reference_epoch());
        builder = builder.with_source(QcNtripSource::new(stream).unwrap());
    }

    let (streamed, arcs) = collect(builder.build().unwrap(), &|rx| {
        rx.to_string()[..4].to_string()
    });

    assert!(arcs > 0, "phase arcs closed at the end of the streams");

    let stations = streamed
        .keys()
        .map(|(station, _, _)| station.clone())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(stations.len(), 2, "{:?}", stations);

    // same lines of sight, same signals, same TEC
    let mut common = 0;
    let mut max_tecu: f64 = 0.0;

    for (key, e) in streamed.iter() {
        let Some(r) = reference.get(key) else {
            continue;
        };
        if (&e.code_1, &e.code_j) != (&r.code_1, &r.code_j) {
            continue;
        }
        common += 1;
        assert!(
            (e.latitude_deg - r.latitude_deg).abs() < 1.0E-6,
            "{:?}",
            key
        );
        assert!(
            (e.longitude_deg - r.longitude_deg).abs() < 1.0E-6,
            "{:?}",
            key
        );
        max_tecu = max_tecu.max((e.vertical_tecu - r.vertical_tecu).abs());
    }

    println!(
        "{} streamed estimates, {} file based, {} in common: max. difference {:.4} TECu",
        streamed.len(),
        reference.len(),
        common,
        max_tecu
    );

    assert!(
        common as f64 > 0.9 * reference.len() as f64,
        "{} estimates in common out of {}",
        common,
        reference.len()
    );

    // RTCM resolution: pseudo ranges 0.6 mm, phases 0.5 mm (×9.52 TECu/m)
    assert!(max_tecu < 0.05, "max. difference {} TECu", max_tecu);
}

/// One connection to a [sessions_caster]
pub enum CasterSession {
    /// The caster closes the connection without any response ("empty response")
    Refuse,
    /// The caster streams these frames, then closes the connection
    Stream(Vec<Vec<u8>>),
    /// The caster accepts the connection and remains silent (station down)
    Silent,
}

/// Local NTRIP caster serving one connection per session, in this order.
/// Returns the port of the caster.
pub fn sessions_caster(sessions: Vec<CasterSession>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    thread::spawn(move || {
        for session in sessions {
            let Ok((mut socket, _)) = listener.accept() else {
                return;
            };

            let mut request = Vec::new();
            let mut buffer = [0u8; 1024];

            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                match socket.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => request.extend_from_slice(&buffer[..n]),
                }
            }

            match session {
                CasterSession::Refuse => drop(socket),
                CasterSession::Silent => {
                    let _ = socket.write_all(b"ICY 200 OK\r\n\r\n");
                    // keeps the connection open, silent
                    thread::spawn(move || {
                        let _socket = socket;
                        thread::sleep(StdDuration::from_secs(600));
                    });
                }
                CasterSession::Stream(frames) => {
                    let _ = socket.write_all(b"ICY 200 OK\r\n\r\n");
                    for (index, frame) in frames.iter().enumerate() {
                        if socket.write_all(frame).is_err() {
                            break;
                        }
                        if index % 32 == 0 {
                            thread::sleep(StdDuration::from_millis(1));
                        }
                    }
                }
            }
        }
    });

    port
}

/// Real-time network: a station refused once by its caster (reconnection), a silent station
/// (maximal latency: not waited for), an unknown mount point (fault tolerance: ended).
/// The session goes on with the other stations.
#[test]
fn ntrip_network_resilience() {
    let end = epoch("2020-06-25T00:30:00 GPST");
    let position = |obs: &str| {
        crate::io::rinex::QcRinexObsSource::antenna_reference_point(&load_rinex(obs)).unwrap()
    };

    let esbc = sessions_caster(vec![CasterSession::Stream(station_frames(
        ESBC_OBS,
        Some(ESBC_NAV),
        Some(position(ESBC_OBS)),
        end,
    ))]);

    let mojn = sessions_caster(vec![
        CasterSession::Refuse,
        CasterSession::Stream(station_frames(
            MOJN_OBS,
            None,
            Some(position(MOJN_OBS)),
            end,
        )),
    ]);

    let silent = sessions_caster(vec![CasterSession::Silent]);
    let unknown = caster("MOUNT", None, vec![]);

    let stream = |port: u16, mount: &str| {
        QcNtripConfig::new("127.0.0.1", port, mount).with_reference_epoch(reference_epoch())
    };

    let clients = [
        QcNtripSource::new(stream(esbc, "ESBC00DNK0")).unwrap(),
        QcNtripSource::new(
            stream(mojn, "MOJN00DNK0").with_reconnection(StdDuration::from_millis(100)),
        )
        .unwrap(),
        QcNtripSource::new(stream(silent, "SILENT")).unwrap(),
        QcNtripSource::new(stream(unknown, "UNKNOWN")).unwrap(),
    ];

    let monitors = clients.iter().map(|c| c.monitor()).collect::<Vec<_>>();

    // the first connection of MOJN is refused
    let start = std::time::Instant::now();
    while monitors[1].last_error().is_none() && start.elapsed() < StdDuration::from_secs(10) {
        thread::sleep(StdDuration::from_millis(1));
    }
    assert!(monitors[1]
        .last_error()
        .is_some_and(|e| e.contains("empty response")));

    let mut builder = QcEngine::<TestOutput>::builder()
        .with_max_latency(Duration::from_seconds(60.0))
        .with_fault_tolerance(true)
        .with_time_window(None, Some(end))
        .with_processor(EpochRecorder::default());

    for client in clients {
        builder = builder.with_source(client);
    }

    let mut engine = builder.build().unwrap();

    let mut epochs = Vec::new();
    for item in engine.by_ref() {
        if let TestOutput::Epoch(summary) = item.unwrap().payload {
            epochs.push(summary);
        }
    }

    let stats = engine.stats();
    println!("{:?}", stats);

    for monitor in monitors.iter() {
        println!(
            "{}: connected {}, connections {}, last error {:?}, {} messages",
            monitor.config(),
            monitor.is_connected(),
            monitor.connections(),
            monitor.last_error(),
            monitor.stats().messages
        );
    }

    // every epoch of ESBC was processed, without waiting for the silent station
    let reference = rinex_epochs(ESBC_OBS, end);
    let esbc = QcIndexing::Custom("ESBC00DNK0".to_string());
    let esbc_epochs = epochs
        .iter()
        .filter(|e| e.receivers.iter().any(|r| r.receiver == esbc))
        .count();
    assert_eq!(esbc_epochs, reference.len());
    assert!(stats.incomplete_epochs > 0);

    // the unknown mount point failed and was ended
    assert_eq!(stats.failed_sources, 1);

    // MOJN: refused once ("empty response"), then reconnected (in the background)
    let start = std::time::Instant::now();
    while monitors[1].stats().msm == 0 && start.elapsed() < StdDuration::from_secs(10) {
        thread::sleep(StdDuration::from_millis(10));
    }
    assert_eq!(monitors[1].connections(), 1);
    assert!(monitors[1].stats().msm > 0);
    // (then the caster closed the connection: the client keeps trying)
    assert!(monitors[1].last_error().is_some());

    // the silent station is connected, nothing received
    assert!(monitors[2].is_connected());
    assert_eq!(monitors[2].stats().messages, 0);
}
