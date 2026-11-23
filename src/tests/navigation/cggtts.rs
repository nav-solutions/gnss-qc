//! CGGTTS scenarios: ESBC00DNK, 2020-06-25 00:00-02:00, GPS, GRG precise products.
use std::io::BufWriter;

use super::*;
use crate::{navigation::QcCggttsError, orbit::SPEED_OF_LIGHT_M_S};

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    Builtin(QcOutput),
}

impl From<QcOutput> for Out {
    fn from(output: QcOutput) -> Self {
        Self::Builtin(output)
    }
}

struct CggttsRun {
    tracks: Vec<CggttsTrack>,
    report: QcCggttsReport,
    solutions: Vec<QcPvtSolution>,
}

fn run_cggtts(method: Method) -> CggttsRun {
    let products = load_products(&QcInputConfig::default(), &[ESBC_OBS, ESBC_NAV, GRG_SP3]);

    let navigation = QcNavigationConfig::default()
        .with_method(method)
        .with_constellations(&[Constellation::GPS])
        .with_user_parameters(UserParameters::new(
            UserProfile::Static,
            ClockProfile::Quartz,
        ));

    let cfg = QcCggttsConfig {
        pvt_solutions: true,
        ..Default::default()
    };

    let processor = QcCggttsProcessor::new(almanac(), navigation, cfg).unwrap();

    let t0 = epoch("2020-06-25T00:00:00 GPST");

    let engine = QcEngine::<Out>::builder()
        .with_products(products)
        .with_time_window(Some(t0), Some(t0 + Duration::from_hours(2.0)))
        .with_processor(processor)
        .build()
        .unwrap();

    let (mut tracks, mut solutions, mut report) = (vec![], vec![], None);

    for item in engine {
        match item.unwrap().payload {
            Out::Builtin(QcOutput::CggttsTrack(track)) => {
                assert_eq!(track.receiver, esbc());
                tracks.push(*track.track);
            }
            Out::Builtin(QcOutput::PVTSolution(solution)) => solutions.push(solution),
            Out::Builtin(QcOutput::CggttsReport(r)) => report = Some(r),
            _ => {}
        }
    }

    CggttsRun {
        tracks,
        report: report.expect("cggtts report"),
        solutions,
    }
}

/// Satellite clock offset from the SP3 (linear interpolation), seconds
fn sp3_clock_s(sp3: &sp3::prelude::SP3, sv: SV, t: Epoch) -> f64 {
    let samples = sp3
        .data
        .iter()
        .filter(|(k, e)| k.sv == sv && e.clock_us.is_some())
        .map(|(k, e)| (k.epoch, e.clock_us.unwrap() * 1.0E-6))
        .collect::<Vec<_>>();

    let pos = samples.partition_point(|(e, _)| *e <= t);
    let ((t0, c0), (t1, c1)) = (samples[pos - 1], samples[pos]);
    c0 + (c1 - c0) * (t - t0).to_seconds() / (t1 - t0).to_seconds()
}

/// CGGTTS in CPP mode (iono free code): tracks, schedule, content.
#[test]
fn cggtts_cpp() {
    let run = run_cggtts(Method::CPP);
    let calendar = CommonViewCalendar::bipm();
    let period = calendar.total_period_duration();

    assert_eq!(run.report.tracks, run.tracks.len());
    assert!(run.tracks.len() > 40, "{:?}", run.report);
    assert!(run.report.short_tracks > 0); // rising and setting satellites

    let sp3 = sp3::prelude::SP3::from_gzip_file(data_path(GRG_SP3)).unwrap();

    for track in run.tracks.iter() {
        // CGGTTS is expressed in UTC
        assert_eq!(track.epoch.time_scale, TimeScale::UTC);

        // complete tracks only, within the 13' data collection
        assert!(
            track.duration >= Duration::from_seconds(720.0),
            "{:?}",
            track
        );
        assert!(
            track.duration <= Duration::from_seconds(780.0),
            "{:?}",
            track
        );

        // BIPM schedule: tracking starts 3' after the period start
        let period_start = calendar.next_period_start_after(track.epoch) - period;
        let offset = track.epoch - period_start;
        assert!(
            offset >= Duration::from_seconds(180.0) && offset < Duration::from_seconds(210.0),
            "{}: not aligned to the schedule ({})",
            track.epoch,
            offset
        );

        // dual frequency GPS code, ionosphere cancelled
        assert_eq!(track.frc, "L3P");
        assert_eq!(track.data.mdio, 0.0);
        assert!(track.data.ioe > 0 || track.sv.prn == 8);

        // REFSYS: receiver clock offset, fitted at midtrack
        let midtrack = track.epoch + track.duration / 2;
        let pvt = run
            .solutions
            .iter()
            .min_by_key(|sol| (sol.solution.epoch.to_time_scale(TimeScale::UTC) - midtrack).abs())
            .unwrap();
        assert!(
            (track.data.refsys - pvt.solution.clock_offset_s).abs() < 5.0E-9,
            "{} refsys={} pvt={}",
            track.sv,
            track.data.refsys,
            pvt.solution.clock_offset_s
        );

        // REFSV = REFSYS - satellite clock (precise products, + relativistic correction)
        let dt_sv = track.data.refsys - track.data.refsv;
        let sp3_clock = sp3_clock_s(&sp3, track.sv, midtrack);
        assert!(
            (dt_sv - sp3_clock).abs() < 50.0E-9,
            "{} dt_sv={} sp3={}",
            track.sv,
            dt_sv,
            sp3_clock
        );

        // MDTR: troposphere delay (s), 2.3 m at zenith, larger at low elevation
        let mdtr_m = track.data.mdtr * SPEED_OF_LIGHT_M_S;
        let zenith_m = mdtr_m * track.elevation_deg.to_radians().sin();
        assert!(
            zenith_m > 2.0 && zenith_m < 2.8,
            "{}: mdtr {} m",
            track.sv,
            mdtr_m
        );

        // receiver clock stability over 13' (DSG): nanosecond level
        assert!(
            track.data.dsg < 3.0E-9,
            "{} dsg={}",
            track.sv,
            track.data.dsg
        );
    }

    // one track per satellite and per period
    for (i, a) in run.tracks.iter().enumerate() {
        for b in run.tracks[i + 1..].iter() {
            assert!(a.sv != b.sv || a.epoch != b.epoch);
        }
    }
}

/// CGGTTS in SPP mode: single frequency (L1 C/A), same scheduling.
#[test]
fn cggtts_spp() {
    let run = run_cggtts(Method::SPP);
    assert!(run.tracks.len() > 40, "{:?}", run.report);

    for track in run.tracks.iter() {
        assert_eq!(track.frc, "L1C");
        assert!(track.data.dsg < 3.0E-9);
    }
}

/// CGGTTS solutions are formatted into a standard CGGTTS file, and parsed back.
#[test]
fn cggtts_file() {
    let run = run_cggtts(Method::PPP);
    assert!(run.tracks.len() > 40, "{:?}", run.report);

    let mut cggtts = CGGTTS::default();
    cggtts.header = cggtts
        .header
        .with_station("ESBC")
        .with_reference_frame("ITRF")
        .with_apc_coordinates(::cggtts::prelude::Coordinates {
            x: 3582105.291,
            y: 532589.731,
            z: 5232754.805,
        });
    cggtts.tracks = run.tracks.clone();

    let mut writer = BufWriter::new(Vec::new());
    cggtts.format(&mut writer).unwrap();
    let content = String::from_utf8(writer.into_inner().unwrap()).unwrap();

    let parsed = CGGTTS::parse(&mut std::io::BufReader::new(content.as_bytes()), false).unwrap();
    assert_eq!(parsed.tracks.len(), run.tracks.len());
    assert_eq!(parsed.header.station, "ESBC");

    for (parsed, track) in parsed.tracks.iter().zip(run.tracks.iter()) {
        assert_eq!(parsed.sv, track.sv);
        // CGGTTS resolution: 0.1 ns
        assert!((parsed.data.refsys - track.data.refsys).abs() < 1.0E-10);
    }
}

/// RTK is not compatible with CGGTTS (no clock state)
#[test]
fn cggtts_rejects_rtk() {
    let navigation = QcNavigationConfig::default().with_rtk_base(mojn());
    let result = QcCggttsProcessor::new(almanac(), navigation, QcCggttsConfig::default());
    assert!(matches!(result, Err(QcCggttsError::RtkNotSupported)));
}
