//! RTCM messages beyond MSM: legacy observations (1004, 1012), Glonass ephemerides (1020),
//! descriptors (1008, 1033), Glonass biases (1230), time tags of real-time streams.
use std::collections::HashMap;

use rinex::prelude::{obs::LliFlags, Observable};

use rtcm_rs::msg::{
    message::Message, GloSigId, Msg1008T, Msg1033T, Msg1084Sig, Msg1084T, Msg1230CodePhaseBias,
    Msg1230T, Msm46Sat,
};

use super::{
    encoder::{frame, glonass_tod_ms, RtcmEncoder},
    epoch, glonass_channels, reference_epoch, rinex_ephemerides, rinex_epochs, ESBC_NAV, ESBC_OBS,
};

use crate::prelude::*;

/// Decodes these messages, returns the data points
fn decode(decoder: &mut QcRtcmDecoder, messages: &[Message]) -> Vec<QcInputItem> {
    let mut items = Vec::new();
    for message in messages.iter() {
        let bytes = frame(message);
        assert_eq!(decoder.decode_bytes(&bytes, &mut items), bytes.len());
    }
    items
}

/// Legacy observations (1004, 1012), real data: same values as the RINEX file
/// (RTCM resolution), Glonass converted with its frequency channels and Glonass time.
#[test]
fn legacy_observations() {
    let end = epoch("2020-06-25T00:30:00 GPST");
    let reference = rinex_epochs(ESBC_OBS, end);

    let mut encoder = RtcmEncoder::default().with_glonass_sbas(&glonass_channels(ESBC_OBS));
    let mut decoder = QcRtcmDecoder::new().with_reference_epoch(reference_epoch());

    let mut offsets = HashMap::<(SV, String), f64>::new();
    let mut count = HashMap::<(Constellation, char), usize>::new();

    for (t, signals) in reference.iter() {
        let messages = encoder.legacy(*t, signals);
        assert_eq!(messages.len(), 2, "1004 and 1012");

        for item in decode(&mut decoder, &messages) {
            assert_eq!(item.epoch, *t, "time tag");

            let QcPayload::Signal(signal) = &item.payload else {
                panic!("unexpected payload {:?}", item.payload);
            };
            let decoded = &signal.signal;

            let original = signals
                .iter()
                .find(|s| s.sv == decoded.sv && s.observable == decoded.observable)
                .unwrap_or_else(|| {
                    panic!("{} {}: {} not in RINEX", t, decoded.sv, decoded.observable)
                });

            let kind = decoded.observable.to_string().chars().next().unwrap();
            *count.entry((decoded.sv.constellation, kind)).or_default() += 1;

            match &decoded.observable {
                // DF011 / DF017 resolution: 0.02 m
                Observable::PseudoRange(_) => assert!(
                    (decoded.value - original.value).abs() <= 0.011,
                    "{} {} {}: {} != {}",
                    t,
                    decoded.sv,
                    decoded.observable,
                    decoded.value,
                    original.value
                ),
                // integer number of cycles (alignment), DF012 resolution: 0.5 mm
                Observable::PhaseRange(code) => {
                    let offset = decoded.value - original.value;
                    assert!(
                        (offset - offset.round()).abs() < 5.0E-3,
                        "{} {} {}: non integer phase offset {}",
                        t,
                        decoded.sv,
                        code,
                        offset
                    );
                    if let Some(previous) =
                        offsets.insert((decoded.sv, code.clone()), offset.round())
                    {
                        if previous != offset.round() {
                            assert!(decoded
                                .lli
                                .is_some_and(|lli| lli.intersects(LliFlags::LOCK_LOSS)));
                        }
                    }
                }
                Observable::SSI(_) => {
                    assert!((decoded.value - original.value).abs() <= 0.125)
                }
                other => panic!("unexpected observable {}", other),
            }
        }
    }

    println!("{:?} {:?}", decoder.stats(), count);

    let stats = decoder.stats();
    assert_eq!(stats.legacy, 2 * reference.len());
    assert_eq!(stats.redundant, 0);
    assert_eq!(stats.msm, 0);

    for constellation in [Constellation::GPS, Constellation::Glonass] {
        for kind in ['C', 'L', 'S'] {
            assert!(
                count
                    .get(&(constellation, kind))
                    .copied()
                    .unwrap_or_default()
                    > 100,
                "{} {}",
                constellation,
                kind
            );
        }
    }
}

/// Legacy and MSM observations streamed together: the legacy observations are skipped
/// once MSM observations of the same constellation were decoded
#[test]
fn legacy_redundant() {
    let end = epoch("2020-06-25T00:05:00 GPST");
    let reference = rinex_epochs(ESBC_OBS, end);

    let mut legacy = RtcmEncoder::default();
    let mut msm = RtcmEncoder::default();
    let mut decoder = QcRtcmDecoder::new().with_reference_epoch(reference_epoch());

    let mut observables = Vec::new();

    for (t, signals) in reference.iter() {
        let mut messages = legacy.legacy(*t, signals);
        messages.extend(msm.msm7(*t, signals));

        for item in decode(&mut decoder, &messages) {
            if let QcPayload::Signal(signal) = &item.payload {
                if signal.signal.sv.constellation == Constellation::GPS {
                    observables.push((
                        item.epoch,
                        signal.signal.sv,
                        signal.signal.observable.clone(),
                    ));
                }
            }
        }
    }

    let stats = decoder.stats();
    assert_eq!(stats.legacy, 1, "first epoch only");
    assert_eq!(stats.redundant, reference.len() - 1);

    // no duplicate observation
    let total = observables.len();
    observables.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
    observables.dedup();
    assert_eq!(observables.len(), total);
}

/// Glonass ephemerides (1020), real data
#[test]
fn glonass_ephemerides() {
    let start = epoch("2020-06-25T00:00:00 GPST");
    let end = epoch("2020-06-25T23:59:59 GPST");

    let (mut decoded_count, mut messages) = (0, 0);

    for (sv, toc, original) in rinex_ephemerides(ESBC_NAV, start, end) {
        if sv.constellation != Constellation::Glonass {
            continue;
        }

        // tb is a time of day: the stream is received around tb
        let mut decoder = QcRtcmDecoder::new().with_reference_epoch(toc);

        let message = RtcmEncoder::glonass_ephemeris(sv, toc, &original).unwrap();
        let items = decode(&mut decoder, &[message]);
        messages += decoder.stats().ephemerides;
        assert_eq!(items.len(), 1);

        let QcPayload::Ephemeris(decoded) = &items[0].payload else {
            panic!("unexpected payload");
        };

        assert_eq!(decoded.sv, sv);

        // ToC: tb (Glonass time of day, 15 minutes multiples): same instant
        assert!(
            (decoded.toc - toc).abs() < Duration::from_seconds(1.0),
            "{} toc {} != {}",
            sv,
            decoded.toc,
            toc
        );

        for (key, resolution) in [
            ("satPosX", 1.0E-3),
            ("satPosY", 1.0E-3),
            ("satPosZ", 1.0E-3),
            ("velX", 1.0E-6),
            ("velY", 1.0E-6),
            ("velZ", 1.0E-6),
            ("accelX", 1.0E-9),
            ("accelY", 1.0E-9),
            ("accelZ", 1.0E-9),
        ] {
            let (a, b) = (
                original.get_orbit_f64(key).unwrap(),
                decoded.ephemeris.get_orbit_f64(key).unwrap(),
            );
            assert!((a - b).abs() < resolution, "{} {}: {} != {}", sv, key, a, b);
        }

        assert!((decoded.ephemeris.clock_bias - original.clock_bias).abs() < 1.0E-9);
        assert!((decoded.ephemeris.clock_drift - original.clock_drift).abs() < 1.0E-12);

        decoded_count += 1;
    }

    assert!(decoded_count > 50, "{} Glonass ephemerides", decoded_count);
    assert_eq!(messages, decoded_count);
}

/// Glonass MSM4 (no frequency channel): phases are delivered once the channel is known (1020)
#[test]
fn glonass_channels_learned() {
    let t = epoch("2020-06-25T00:00:00 GPST");

    let msm4 = |t: Epoch| {
        let mut msg = Msg1084T {
            glo_epoch_time_ms: glonass_tod_ms(t),
            ..Default::default()
        };
        msg.data_segment.satellite_data.push(Msm46Sat {
            satellite_id: 1,
            gnss_satellite_rough_range_integer_ms: Some(70),
            gnss_satellite_rough_range_mod1ms_ms: 0.5,
        });
        msg.data_segment.signal_data.push(Msg1084Sig {
            satellite_id: 1,
            signal_id: GloSigId::new(1, 'C'),
            gnss_signal_fine_pseudorange_ms: Some(1.0E-4),
            gnss_signal_fine_phaserange_ms: Some(1.0E-4),
            gnss_phaserange_lock_time_ind: 10,
            half_cycle_ambiguity_ind: 0,
            gnss_signal_cnr_dbhz: Some(45),
        });
        Message::Msg1084(msg)
    };

    let observables = |items: &[QcInputItem], t: Epoch| {
        items
            .iter()
            .filter_map(|item| match &item.payload {
                QcPayload::Signal(s) => {
                    assert_eq!(item.epoch, t, "Glonass time");
                    Some(s.signal.observable.to_string())
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };

    let mut decoder = QcRtcmDecoder::new().with_reference_epoch(reference_epoch());

    // channel unknown: pseudo range and C/N0 only
    assert_eq!(
        observables(&decode(&mut decoder, &[msm4(t)]), t),
        ["C1C", "S1C"]
    );

    // ephemeris of R01: channel 1
    let ephemeris = rtcm_rs::msg::Msg1020T {
        glo_satellite_id: 1,
        glo_satellite_freq_chan_number: 1,
        tb_min: 180,
        glo_m_n4_year: 2020,
        ..Default::default()
    };
    decode(&mut decoder, &[Message::Msg1020(ephemeris)]);

    let t = t + Duration::from_seconds(1.0);
    let items = decode(&mut decoder, &[msm4(t)]);
    assert_eq!(observables(&items, t), ["C1C", "L1C", "S1C"]);

    // phase in cycles of the channel 1 frequency
    let phase = items
        .iter()
        .find_map(|item| match &item.payload {
            QcPayload::Signal(s) if s.signal.observable.to_string() == "L1C" => {
                Some(s.signal.value)
            }
            _ => None,
        })
        .unwrap();

    let range_m = (70.5 + 1.0E-4) * 299_792.458;
    let wavelength_m = 299_792_458.0 / (1602.0E6 + 562.5E3);
    assert!((phase - range_m / wavelength_m).abs() < 1.0E-3);
}

/// Antenna and receiver descriptors (1008, 1033), Glonass code-phase biases (1230)
#[test]
fn descriptors_and_biases() {
    let mut decoder = QcRtcmDecoder::new().with_reference_epoch(reference_epoch());

    let msg1008 = Msg1008T {
        antenna_descriptor_str: "TRM59800.00     NONE".into(),
        antenna_serial_number_str: "5000116001".into(),
        ..Default::default()
    };

    let msg1033 = Msg1033T {
        antenna_descriptor_str: "TRM59800.00     NONE".into(),
        antenna_serial_number_str: "5000116001".into(),
        receiver_type_descriptor_str: "SEPT POLARX5".into(),
        receiver_firmware_version_str: "5.3.2".into(),
        receiver_serial_number_str: "3047937".into(),
        ..Default::default()
    };

    let mut msg1230 = Msg1230T {
        glo_code_phase_bias_ind: 1,
        ..Default::default()
    };
    for (band, attribute, bias) in [(1, 'C', -0.71), (2, 'P', 1.02)] {
        msg1230.glo_code_phase_biases.push(Msg1230CodePhaseBias {
            signal_id: GloSigId::new(band, attribute),
            bias_m: bias,
        });
    }

    let items = decode(
        &mut decoder,
        &[
            Message::Msg1008(msg1008),
            Message::Msg1033(msg1033),
            Message::Msg1230(msg1230),
        ],
    );

    assert!(items.is_empty(), "metadata only");

    let stats = decoder.stats();
    assert_eq!(
        stats.antenna.as_deref(),
        Some("TRM59800.00     NONE 5000116001")
    );
    assert_eq!(
        stats.receiver.as_deref(),
        Some("SEPT POLARX5 5.3.2 3047937")
    );
    assert_eq!(stats.glonass_code_phase_biases.len(), 2);
    assert!((stats.glonass_code_phase_biases["1C"] + 0.71).abs() < 0.02);
    assert!((stats.glonass_code_phase_biases["2P"] - 1.02).abs() < 0.02);
    assert!(stats.ignored.is_empty(), "{:?}", stats.ignored);
}

/// Real-time stream (no reference epoch): the station coordinates and ephemerides received
/// before the first observation are tagged with its epoch, the stream remains time ordered
#[test]
fn context_before_observations() {
    let end = epoch("2020-06-25T00:00:30 GPST");
    let reference = rinex_epochs(ESBC_OBS, end);

    let mut encoder = RtcmEncoder::default();

    // decoder in real-time mode, without reference: the week of the observations is resolved
    // against the system time, the time of week is preserved
    let mut decoder = QcRtcmDecoder::new();

    let ephemeris = rinex_ephemerides(ESBC_NAV, epoch("2020-06-24T22:00:00 GPST"), end)
        .into_iter()
        .find_map(|(sv, toc, eph)| RtcmEncoder::ephemeris(sv, toc, &eph))
        .unwrap();

    let mut messages = vec![
        RtcmEncoder::station((3582105.29, 532589.73, 5232754.80)),
        ephemeris,
    ];
    let held = decode(&mut decoder, &messages);
    assert!(held.is_empty(), "held until the first observation");

    messages = encoder.msm7(reference[0].0, &reference[0].1);
    let items = decode(&mut decoder, &messages);

    assert!(matches!(items[0].payload, QcPayload::ReferencePosition(_)));
    assert!(matches!(items[1].payload, QcPayload::Ephemeris(_)));
    assert!(matches!(items[2].payload, QcPayload::Signal(_)));

    // time ordered: same epoch as the first observation
    let first = items[2].epoch;
    assert_eq!(items[0].epoch, first);
    assert_eq!(items[1].epoch, first);
    assert_eq!(
        first.to_time_of_week().1,
        reference[0].0.to_time_of_week().1,
        "time of week"
    );
    assert!(items.windows(2).all(|w| w[0].epoch <= w[1].epoch));

    // next coordinates: tagged with the latest observation
    let items = decode(
        &mut decoder,
        &[RtcmEncoder::station((3582105.29, 532589.73, 5232754.80))],
    );
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].epoch, first);
}
