//! RTCM decoding, verified on real data: RINEX observations and ephemerides are encoded
//! as RTCM 3 frames (test encoder), decoded by [QcRtcmDecoder] and compared to the originals.
pub mod encoder;

#[cfg(feature = "ntrip")]
mod ntrip;

mod messages;

use std::collections::HashMap;

use rinex::prelude::{obs::SignalObservation, Observable};

use crate::{
    io::rinex::{QcRinexNavSource, QcRinexObsSource},
    orbit::QcKepler,
    tests::miso::{data_path, load_rinex},
};

use crate::prelude::*;

use encoder::{frame, RtcmEncoder};

pub const ESBC_OBS: &str = "CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz";
pub const ESBC_NAV: &str = "NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz";

pub fn epoch(s: &str) -> Epoch {
    use std::str::FromStr;
    Epoch::from_str(s).unwrap()
}

/// Approximate date of the recorded streams
pub fn reference_epoch() -> Epoch {
    epoch("2020-06-25T00:00:00 GPST")
}

/// Signal observations of a RINEX file, epoch by epoch, up to `end`
pub fn rinex_epochs(file: &str, end: Epoch) -> Vec<(Epoch, Vec<SignalObservation>)> {
    let rinex = load_rinex(file);
    let mut source = QcRinexObsSource::new(QcIndexing::Custom("RINEX".to_string()), rinex).unwrap();

    let mut epochs: Vec<(Epoch, Vec<SignalObservation>)> = Vec::new();

    while let QcPoll::Ready(item) = source.poll().unwrap() {
        if item.epoch > end {
            break;
        }
        if let QcPayload::Signal(signal) = item.payload {
            match epochs.last_mut() {
                Some((t, signals)) if *t == item.epoch => signals.push(signal.signal),
                _ => epochs.push((item.epoch, vec![signal.signal])),
            }
        }
    }

    epochs
}

/// Radio ephemerides of a RINEX file, whose ToC lies within [start, end]
pub fn rinex_ephemerides(
    file: &str,
    start: Epoch,
    end: Epoch,
) -> Vec<(SV, Epoch, rinex::navigation::Ephemeris)> {
    let rinex = load_rinex(file);
    let mut source = QcRinexNavSource::new(QcIndexing::Custom("NAV".to_string()), rinex).unwrap();

    let mut ephemerides = Vec::new();

    while let QcPoll::Ready(item) = source.poll().unwrap() {
        if let QcPayload::Ephemeris(eph) = item.payload {
            if eph.toc >= start && eph.toc <= end {
                ephemerides.push((eph.sv, eph.toc, *eph.ephemeris));
            }
        }
    }

    ephemerides
}

/// RTCM frames of a station: coordinates, ephemerides (optional), then the observations
pub fn station_frames(
    obs: &str,
    nav: Option<&str>,
    position_ecef_m: Option<(f64, f64, f64)>,
    end: Epoch,
) -> Vec<Vec<u8>> {
    let mut frames = Vec::new();

    if let Some(position) = position_ecef_m {
        frames.push(frame(&RtcmEncoder::station(position)));
    }

    if let Some(nav) = nav {
        let start = reference_epoch() - Duration::from_hours(4.0);
        for (sv, toc, eph) in rinex_ephemerides(nav, start, end + Duration::from_hours(4.0)) {
            if let Some(message) = RtcmEncoder::ephemeris(sv, toc, &eph) {
                frames.push(frame(&message));
            }
        }
    }

    let mut encoder = RtcmEncoder::default();

    for (t, signals) in rinex_epochs(obs, end) {
        for message in encoder.msm7(t, &signals) {
            frames.push(frame(&message));
        }
    }

    frames
}

/// Frequency channels of the Glonass satellites (RINEX header)
pub fn glonass_channels(file: &str) -> HashMap<SV, i8> {
    load_rinex(file).header.glo_channels.clone()
}

/// MSM7 of all constellations (GPS, Glonass, Galileo, SBAS, QZSS, BeiDou)
#[test]
fn msm7_observations() {
    let end = epoch("2020-06-25T00:30:00 GPST");
    let reference = rinex_epochs(ESBC_OBS, end);

    let mut encoder = RtcmEncoder::default().with_glonass_sbas(&glonass_channels(ESBC_OBS));
    let mut decoder = QcRtcmDecoder::new().with_reference_epoch(reference_epoch());

    // phase offsets (integer cycles) introduced by the alignment on the pseudo ranges
    let mut offsets = HashMap::<(SV, String), f64>::new();

    let (mut pseudo_ranges, mut phases, mut dopplers, mut snrs) = (0, 0, 0, 0);
    let mut decoded_svs = Vec::new();

    for (t, signals) in reference.iter() {
        let mut items = Vec::new();
        let mut bytes = Vec::new();

        for message in encoder.msm7(*t, signals) {
            bytes.extend(frame(&message));
        }

        // byte stream decoding
        assert_eq!(decoder.decode_bytes(&bytes, &mut items), bytes.len());

        for item in items.iter() {
            assert_eq!(item.epoch, *t, "invalid time tag");

            let decoded = match &item.payload {
                QcPayload::Signal(signal) => &signal.signal,
                other => panic!("unexpected payload {:?}", other),
            };

            decoded_svs.push(decoded.sv);

            let original = signals
                .iter()
                .find(|s| s.sv == decoded.sv && s.observable == decoded.observable)
                .unwrap_or_else(|| {
                    panic!("{} {}: {} not in RINEX", t, decoded.sv, decoded.observable)
                });

            match &decoded.observable {
                Observable::PseudoRange(_) => {
                    pseudo_ranges += 1;
                    assert!(
                        (decoded.value - original.value).abs() < 1.0E-3,
                        "{} {} {}: {} != {}",
                        t,
                        decoded.sv,
                        decoded.observable,
                        decoded.value,
                        original.value
                    );
                }
                Observable::PhaseRange(code) => {
                    phases += 1;
                    let offset = decoded.value - original.value;
                    assert!(
                        (offset - offset.round()).abs() < 2.0E-3,
                        "{} {} {}: non integer phase offset {}",
                        t,
                        decoded.sv,
                        code,
                        offset
                    );
                    let previous = offsets.insert((decoded.sv, code.clone()), offset.round());
                    if let Some(previous) = previous {
                        if previous != offset.round() {
                            // realignment: must be flagged
                            assert!(decoded.lli.is_some_and(
                                |lli| lli.intersects(rinex::prelude::obs::LliFlags::LOCK_LOSS)
                            ));
                        }
                    }
                }
                Observable::Doppler(_) => {
                    dopplers += 1;
                    assert!(
                        (decoded.value - original.value).abs() < 1.0E-3 / 0.19,
                        "{} {} {}: {} != {}",
                        t,
                        decoded.sv,
                        decoded.observable,
                        decoded.value,
                        original.value
                    );
                }
                Observable::SSI(_) => {
                    snrs += 1;
                    assert!((decoded.value - original.value).abs() <= 1.0 / 32.0);
                }
                other => panic!("unexpected observable {}", other),
            }
        }
    }

    let stats = decoder.stats();
    println!("{:?}", stats);
    println!(
        "pseudo ranges: {}, phases: {}, dopplers: {}, C/N0: {}",
        pseudo_ranges, phases, dopplers, snrs
    );

    // RINEX observations that MSM messages can carry
    let eligible = |prefix: char| {
        reference
            .iter()
            .flat_map(|(_, signals)| signals.iter())
            .filter(|s| {
                let code = s.observable.to_string();
                code.starts_with(prefix) && RtcmEncoder::is_valid(s.sv.constellation, &code[1..])
            })
            .count()
    };

    let (eligible_pr, eligible_ph, eligible_dop, eligible_snr) =
        (eligible('C'), eligible('L'), eligible('D'), eligible('S'));

    println!(
        "RINEX: pseudo ranges: {}, phases: {}, dopplers: {}, C/N0: {}",
        eligible_pr, eligible_ph, eligible_dop, eligible_snr
    );

    assert_eq!(reference.len(), 61);

    // nothing is lost
    assert_eq!(pseudo_ranges, eligible_pr);
    assert_eq!(phases, eligible_ph);
    assert_eq!(dopplers, eligible_dop);
    assert_eq!(snrs, eligible_snr);
    assert_eq!(stats.signals, pseudo_ranges + phases + dopplers + snrs);
    assert!(stats.ignored.is_empty());

    // every constellation in sight is decoded
    let constellations = |signals: &mut dyn Iterator<Item = SV>| {
        let mut constellations = signals.map(|sv| sv.constellation).collect::<Vec<_>>();
        constellations.sort();
        constellations.dedup();
        constellations
    };

    let in_sight = constellations(&mut reference.iter().flat_map(|(_, signals)| {
        signals
            .iter()
            .filter(|s| RtcmEncoder::is_valid(s.sv.constellation, &s.observable.to_string()[1..]))
            .map(|s| s.sv)
    }));

    assert_eq!(constellations(&mut decoded_svs.into_iter()), in_sight);
    assert!(in_sight.len() >= 3, "{:?}", in_sight);

    // Glonass (FDMA, Glonass time) and SBAS are part of it
    assert!(in_sight.contains(&Constellation::Glonass), "{:?}", in_sight);
    assert!(in_sight.iter().any(|c| c.is_sbas()), "{:?}", in_sight);
}

#[test]
fn radio_ephemerides() {
    let start = epoch("2020-06-25T00:00:00 GPST");
    let end = epoch("2020-06-25T23:59:59 GPST");
    let ephemerides = rinex_ephemerides(ESBC_NAV, start, end);

    let mut decoder = QcRtcmDecoder::new().with_reference_epoch(reference_epoch());
    let mut per_constellation = HashMap::<Constellation, usize>::new();

    for (sv, toc, original) in ephemerides.iter() {
        let message = match RtcmEncoder::ephemeris(*sv, *toc, original) {
            Some(message) => message,
            None => continue,
        };

        let mut items = Vec::new();
        decoder.decode_bytes(&frame(&message), &mut items);
        assert_eq!(items.len(), 1);

        let decoded = match &items[0].payload {
            QcPayload::Ephemeris(eph) => eph,
            other => panic!("unexpected payload {:?}", other),
        };

        // reception time
        assert_eq!(items[0].epoch, reference_epoch());

        assert_eq!(decoded.sv, *sv);
        assert_eq!(decoded.toc, *toc, "{} ToC", sv);
        assert_eq!(decoded.toe(), original.toe(*sv), "{} ToE", sv);
        assert_eq!(
            decoded.ephemeris.sv_healthy(),
            original.sv_healthy(),
            "{} health",
            sv
        );

        // same satellite states (RTCM resolution)
        let kepler = QcKepler::from_ephemeris(*sv, *toc, original).unwrap();
        let decoded_kepler =
            QcKepler::from_ephemeris(*sv, decoded.toc, &decoded.ephemeris).unwrap();

        for dt in [0.0, 900.0, 3600.0] {
            let t = original.toe(*sv).unwrap() + Duration::from_seconds(dt);
            let (a, b) = (kepler.state(t), decoded_kepler.state(t));

            let error_m = ((a.position_m.0 - b.position_m.0).powi(2)
                + (a.position_m.1 - b.position_m.1).powi(2)
                + (a.position_m.2 - b.position_m.2).powi(2))
            .sqrt()
                * 1.0;

            assert!(error_m < 0.05, "{} {} position error {} m", sv, t, error_m);

            let clock_error_s = (kepler.clock_offset_s(t) - decoded_kepler.clock_offset_s(t)).abs();
            assert!(
                clock_error_s < 1.0E-10,
                "{} {} clock error {} s",
                sv,
                t,
                clock_error_s
            );
        }

        *per_constellation.entry(sv.constellation).or_default() += 1;
    }

    println!("{:?}", per_constellation);

    for constellation in [
        Constellation::GPS,
        Constellation::Galileo,
        Constellation::BeiDou,
        Constellation::QZSS,
    ] {
        assert!(
            per_constellation
                .get(&constellation)
                .copied()
                .unwrap_or_default()
                > 0,
            "no {} ephemeris",
            constellation
        );
    }
}

#[test]
fn station_coordinates() {
    let position = (3582105.2910, 532589.7313, 5232754.8054);

    let mut decoder = QcRtcmDecoder::new().with_reference_epoch(reference_epoch());
    let mut items = Vec::new();

    decoder.decode_bytes(&frame(&RtcmEncoder::station(position)), &mut items);

    assert_eq!(items.len(), 1);
    match items[0].payload {
        QcPayload::ReferencePosition((x, y, z)) => {
            assert!((x - position.0).abs() < 1.0E-4);
            assert!((y - position.1).abs() < 1.0E-4);
            assert!((z - position.2).abs() < 1.0E-4);
        }
        ref other => panic!("unexpected payload {:?}", other),
    }
}

#[test]
fn week_rollover() {
    let signals = rinex_epochs(ESBC_OBS, reference_epoch())
        .into_iter()
        .next()
        .unwrap()
        .1;

    // same time of week: the week is resolved against the reference (closest occurence)
    for (reference, expected) in [
        // nominal
        ("2020-06-24T23:00:00 GPST", "2020-06-25T00:00:00 GPST"),
        // reference 3 days earlier
        ("2020-06-22T01:00:00 GPST", "2020-06-25T00:00:00 GPST"),
        // reference 3 days later, in the next week
        ("2020-06-28T00:00:10 GPST", "2020-06-25T00:00:00 GPST"),
        // reference 5 days earlier: the previous week is closer
        ("2020-06-20T00:00:00 GPST", "2020-06-18T00:00:00 GPST"),
        // reference one week later: the closest occurence
        ("2020-07-02T00:00:00 GPST", "2020-07-02T00:00:00 GPST"),
    ] {
        let mut encoder = RtcmEncoder::default();
        let mut decoder = QcRtcmDecoder::new().with_reference_epoch(epoch(reference));

        let mut items = Vec::new();
        for message in encoder.msm7(reference_epoch(), &signals) {
            decoder.decode(&message, &mut items);
        }

        assert!(!items.is_empty());
        for item in items.iter() {
            assert_eq!(item.epoch, epoch(expected), "reference {}", reference);
        }
    }
}

/// Recorded RTCM stream (file), processed by the engine:
/// same epochs and signals as the RINEX file it was produced from.
#[test]
fn recorded_stream() {
    let end = epoch("2020-06-25T01:00:00 GPST");

    let mut bytes = Vec::new();
    for frame in station_frames(ESBC_OBS, Some(ESBC_NAV), None, end) {
        bytes.extend(frame);
    }

    let esbc = QcIndexing::Custom("ESBC".to_string());

    let stream = QcRtcmSource::new(
        QcSourceDescriptor::new(esbc.clone())
            .with_signals()
            .with_ephemerides(),
        QcRtcmDecoder::new().with_reference_epoch(reference_epoch()),
        bytes,
    );

    let engine = QcEngine::<crate::tests::miso::TestOutput>::builder()
        .with_source(stream)
        .with_processor(crate::tests::miso::EpochRecorder::default())
        .build()
        .unwrap();

    let mut epochs = Vec::new();
    for item in engine {
        if let crate::tests::miso::TestOutput::Epoch(summary) = item.unwrap().payload {
            epochs.push(summary);
        }
    }

    let reference = rinex_epochs(ESBC_OBS, end);
    assert_eq!(epochs.len(), reference.len());

    for (summary, (t, signals)) in epochs.iter().zip(reference.iter()) {
        assert_eq!(summary.epoch, *t);
        assert_eq!(summary.receivers.len(), 1);
        assert_eq!(summary.receivers[0].receiver, esbc);

        // satellites of the supported constellations
        let mut satellites = signals
            .iter()
            .map(|s| s.sv)
            .filter(|sv| {
                matches!(
                    sv.constellation,
                    Constellation::GPS
                        | Constellation::Galileo
                        | Constellation::BeiDou
                        | Constellation::QZSS
                )
            })
            .collect::<Vec<_>>();
        satellites.sort();
        satellites.dedup();

        assert_eq!(summary.receivers[0].satellites, satellites, "{}", t);

        // ephemerides were delivered before the observations
        assert!(summary.buffered_ephemerides > 0);
    }

    let _ = data_path(ESBC_OBS);
}
