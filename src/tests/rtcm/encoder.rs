//! Test RTCM encoder: RINEX observations, ephemerides and station coordinates
//! converted to RTCM 3 frames (MSM7, 1019, 1042, 1044, 1045, 1046, 1005), to stream real data
//! through the RTCM decoder and the NTRIP clients.
use std::collections::{BTreeMap, HashMap};

use rinex::{
    navigation::Ephemeris,
    prelude::{obs::SignalObservation, Observable},
};

use rtcm_rs::{
    msg::{
        message::Message, BdsSigId, GalSigId, GpsSigId, Msg1005T, Msg1019T, Msg1042T, Msg1044T,
        Msg1045T, Msg1046T, Msg1077T, Msg1097T, Msg1117T, Msg1127T, QzssSigId,
    },
    MessageBuilder,
};

use crate::prelude::*;

const LIGHT_MS: f64 = 299_792.458;

/// MSM cell: all observations of one signal
#[derive(Default, Clone)]
struct Cell {
    pseudo_range_m: Option<f64>,
    phase_cycles: Option<f64>,
    doppler_hz: Option<f64>,
    cnr_dbhz: Option<f64>,
    lock_loss: bool,
}

/// RINEX to RTCM encoder
#[derive(Default)]
pub struct RtcmEncoder {
    /// Integer number of cycles added to the RINEX phase, to align it with the pseudo range
    /// (as RTCM requires), per signal
    phase_offsets: HashMap<(SV, String), f64>,
    /// Lock time indicators, per signal
    locks: HashMap<(SV, String), u16>,
}

/// Encodes one message as a frame
pub fn frame(message: &Message) -> Vec<u8> {
    let mut builder = MessageBuilder::new();
    builder
        .build_message(message)
        .unwrap_or_else(|e| panic!("failed to encode {:?}: {:?}", message.number(), e))
        .to_vec()
}

/// GPS time of week (ms)
fn gps_tow_ms(epoch: Epoch) -> u32 {
    let (_, nanos) = epoch.to_time_scale(TimeScale::GPST).to_time_of_week();
    (nanos / 1_000_000) as u32
}

impl RtcmEncoder {
    /// MSM7 messages of one epoch (GPS, Galileo, BeiDou and QZSS signals)
    pub fn msm7(&mut self, epoch: Epoch, signals: &[SignalObservation]) -> Vec<Message> {
        // constellation > satellite > signal code ("1C")
        let mut cells: BTreeMap<Constellation, BTreeMap<u8, BTreeMap<String, Cell>>> =
            BTreeMap::new();

        for signal in signals.iter() {
            let constellation = signal.sv.constellation;
            if !matches!(
                constellation,
                Constellation::GPS
                    | Constellation::Galileo
                    | Constellation::BeiDou
                    | Constellation::QZSS
            ) {
                continue;
            }

            let code = signal.observable.to_string();
            let code = code[1..].to_string();

            if !Self::is_valid(constellation, &code) {
                continue;
            }

            let cell = cells
                .entry(constellation)
                .or_default()
                .entry(signal.sv.prn)
                .or_default()
                .entry(code)
                .or_default();

            match &signal.observable {
                Observable::PseudoRange(_) => cell.pseudo_range_m = Some(signal.value),
                Observable::PhaseRange(_) => {
                    cell.phase_cycles = Some(signal.value);
                    if let Some(lli) = signal.lli {
                        cell.lock_loss |= lli.intersects(rinex::prelude::obs::LliFlags::LOCK_LOSS);
                    }
                }
                Observable::Doppler(_) => cell.doppler_hz = Some(signal.value),
                Observable::SSI(_) => cell.cnr_dbhz = Some(signal.value),
                _ => {}
            }
        }

        let mut messages = Vec::new();
        let nb_constellations = cells.len();

        for (index, (constellation, satellites)) in cells.into_iter().enumerate() {
            // MSM limit: 64 cells (satellites x signals) per message
            let nb_signals = satellites
                .values()
                .flat_map(|signals| signals.keys())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                .max(1);

            let chunk = (64 / nb_signals).max(1);
            let satellites = satellites.into_iter().collect::<Vec<_>>();
            let nb_chunks = satellites.len().div_ceil(chunk);

            for (chunk_index, satellites) in satellites.chunks(chunk).enumerate() {
                let last = index == nb_constellations - 1 && chunk_index == nb_chunks - 1;
                if let Some(message) = self.msm7_message(epoch, constellation, satellites, !last) {
                    messages.push(message);
                }
            }
        }

        messages
    }

    /// Returns true if this signal ("1C") can be encoded in MSM messages
    pub fn is_valid(constellation: Constellation, code: &str) -> bool {
        let mut chars = code.chars();
        let (Some(band), Some(attribute)) = (chars.next(), chars.next()) else {
            return false;
        };
        let Some(band) = band.to_digit(10) else {
            return false;
        };
        let band = band as u8;
        match constellation {
            Constellation::GPS => GpsSigId::new(band, attribute).is_valid(),
            Constellation::Galileo => GalSigId::new(band, attribute).is_valid(),
            Constellation::BeiDou => BdsSigId::new(band, attribute).is_valid(),
            Constellation::QZSS => QzssSigId::new(band, attribute).is_valid(),
            _ => false,
        }
    }

    fn msm7_message(
        &mut self,
        epoch: Epoch,
        constellation: Constellation,
        satellites: &[(u8, BTreeMap<String, Cell>)],
        multiple: bool,
    ) -> Option<Message> {
        macro_rules! build {
            ($type:ident, $message:ident, $variant:ident, $sig_id:ident, $time_field:ident, $tow:expr) => {{
                let mut msg = $type::default();
                msg.reference_station_id = 1;
                msg.$time_field = $tow;
                msg.msm_multiple_message_flag = multiple as u8;

                for (prn, signals) in satellites.iter() {
                    let sv = SV::new(constellation, *prn);

                    // rough range: median pseudo range (robust to an inconsistent signal),
                    // 1/1024 ms resolution
                    let mut pseudo_ranges = signals
                        .values()
                        .filter_map(|cell| cell.pseudo_range_m)
                        .collect::<Vec<_>>();
                    pseudo_ranges.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let reference_ms = pseudo_ranges.get(pseudo_ranges.len() / 2)? / LIGHT_MS;
                    // (rounded: the fine ranges are centered, ±2^-11 ms)
                    let rough_ms = (reference_ms * 1024.0).round() / 1024.0;

                    // rough phase range rate: first doppler
                    let rough_rate = signals.iter().find_map(|(code, cell)| {
                        let doppler = cell.doppler_hz?;
                        let wavelength = Observable::PhaseRange(format!("L{}", code))
                            .to_carrier(constellation)
                            .ok()?
                            .wavelength();
                        Some((-doppler * wavelength).round())
                    });

                    let mut sat = rtcm_rs::msg::$variant::default();
                    sat.satellite_id = *prn;
                    sat.gnss_satellite_rough_range_integer_ms = Some(rough_ms.floor() as u8);
                    sat.gnss_satellite_rough_range_mod1ms_ms = rough_ms - rough_ms.floor();
                    sat.gnss_satellite_rough_phaserange_rates_m_s = rough_rate.map(|r| r as i16);
                    msg.data_segment.satellite_data.push(sat);

                    for (code, cell) in signals.iter() {
                        let mut chars = code.chars();
                        let band = chars.next().unwrap().to_digit(10).unwrap() as u8;
                        let attribute = chars.next().unwrap();

                        let wavelength = Observable::PhaseRange(format!("L{}", code))
                            .to_carrier(constellation)
                            .ok()
                            .map(|carrier| carrier.wavelength());

                        let key = (sv, code.clone());

                        let mut sig = rtcm_rs::msg::$sig_id::default();
                        sig.satellite_id = *prn;
                        sig.signal_id = <rtcm_rs::msg::$sig_id as SigIdOf>::sig_id(band, attribute);

                        // DF405 range: ±2^-10 ms (invalid beyond, like a receiver would do)
                        sig.gnss_signal_fine_pseudorange_ext_ms = cell
                            .pseudo_range_m
                            .map(|pr| pr / LIGHT_MS - rough_ms)
                            .filter(|fine| fine.abs() < 1.0 / 1024.0);

                        sig.gnss_signal_fine_phaserange_ext_ms =
                            match (cell.phase_cycles, wavelength) {
                                (Some(cycles), Some(wavelength)) => {
                                    let pr_m =
                                        cell.pseudo_range_m.unwrap_or(reference_ms * LIGHT_MS);

                                    // align the phase with the pseudo range (integer cycles),
                                    // when the signal is first seen or after a loss of lock
                                    let offset = match self.phase_offsets.get(&key) {
                                        Some(offset) if !cell.lock_loss => *offset,
                                        _ => {
                                            let offset =
                                                ((pr_m - cycles * wavelength) / wavelength).round();
                                            self.phase_offsets.insert(key.clone(), offset);
                                            offset
                                        }
                                    };

                                    // DF406 range: ±2^-8 ms
                                    Some((cycles + offset) * wavelength / LIGHT_MS - rough_ms)
                                        .filter(|fine| fine.abs() < 1.0 / 256.0)
                                }
                                _ => None,
                            };

                        // lock time indicator: grows with the tracking time
                        let lock = self.locks.entry(key).or_insert(0);
                        if cell.lock_loss {
                            *lock = 0;
                        } else {
                            *lock = (*lock + 1).min(1023);
                        }
                        sig.gnss_phaserange_lock_time_ext_ind = *lock;

                        sig.gnss_signal_cnr_ext_dbhz = cell.cnr_dbhz;

                        sig.gnss_signal_fine_phaserange_rate_m_s =
                            match (cell.doppler_hz, wavelength, rough_rate) {
                                (Some(doppler), Some(wavelength), Some(rough)) => {
                                    Some(-doppler * wavelength - rough)
                                }
                                _ => None,
                            };

                        msg.data_segment.signal_data.push(sig);
                    }
                }

                Message::$message(msg)
            }};
        }

        let tow = gps_tow_ms(epoch);

        Some(match constellation {
            Constellation::GPS => build!(
                Msg1077T,
                Msg1077,
                Msm57Sat,
                Msg1077Sig,
                gps_epoch_time_ms,
                tow
            ),
            Constellation::Galileo => {
                build!(
                    Msg1097T,
                    Msg1097,
                    Msm57Sat,
                    Msg1097Sig,
                    gal_epoch_time_ms,
                    tow
                )
            }
            Constellation::QZSS => {
                build!(
                    Msg1117T,
                    Msg1117,
                    Msm57Sat,
                    Msg1117Sig,
                    qzss_epoch_time_ms,
                    tow
                )
            }
            Constellation::BeiDou => build!(
                Msg1127T,
                Msg1127,
                Msm57Sat,
                Msg1127Sig,
                bds_epoch_time_ms,
                (tow as i64 - 14_000).rem_euclid(604_800_000) as u32
            ),
            _ => return None,
        })
    }

    /// Station coordinates (1005)
    pub fn station(position_ecef_m: (f64, f64, f64)) -> Message {
        Message::Msg1005(Msg1005T {
            reference_station_id: 1,
            gps_flag: 1,
            galileo_flag: 1,
            antenna_ref_point_ecef_x_m: position_ecef_m.0,
            antenna_ref_point_ecef_y_m: position_ecef_m.1,
            antenna_ref_point_ecef_z_m: position_ecef_m.2,
            ..Default::default()
        })
    }

    /// Radio ephemeris message (1019, 1042, 1044, 1045 or 1046)
    pub fn ephemeris(sv: SV, toc: Epoch, eph: &Ephemeris) -> Option<Message> {
        let f = |key: &str| eph.get_orbit_f64(key);
        let sc = |key: &str| eph.get_orbit_f64(key).map(|rad| rad / std::f64::consts::PI);
        let u = |key: &str| eph.orbits.get(key).map(|item| item.as_u32());

        let (_, toc_nanos) = toc.to_time_of_week();
        let toc_s = toc_nanos as f64 * 1.0E-9;

        macro_rules! kepler {
            ($msg:expr) => {{
                $msg.crs_m = f("crs")? as _;
                $msg.delta_n_sc_s = sc("deltaN")? as _;
                $msg.m0_sc = sc("m0")? as _;
                $msg.cuc_rad = f("cuc")? as _;
                $msg.eccentricity = f("e")? as _;
                $msg.cus_rad = f("cus")? as _;
                $msg.sqrt_a_sqrt_m = f("sqrta")? as _;
                $msg.toe_s = f("toe")? as _;
                $msg.cic_rad = f("cic")? as _;
                $msg.omega0_sc = sc("omega0")? as _;
                $msg.cis_rad = f("cis")? as _;
                $msg.i0_sc = sc("i0")? as _;
                $msg.crc_m = f("crc")? as _;
                $msg.omega_sc = sc("omega")? as _;
                $msg.omegadot_sc_s = sc("omegaDot")? as _;
                $msg.idot_sc_s = sc("idot")? as _;
            }};
        }

        match sv.constellation {
            Constellation::GPS => {
                let mut msg = Msg1019T {
                    gps_satellite_id: sv.prn,
                    gps_week_number: (u("week")? % 1024) as u16,
                    iode: f("iode")? as u8,
                    iodc: f("iodc")? as u16,
                    toc_s: toc_s as f32,
                    af0_s: eph.clock_bias,
                    af1_s_s: eph.clock_drift as f32,
                    af2_s_s2: eph.clock_drift_rate as f32,
                    tgd_s: f("tgd").unwrap_or_default() as f32,
                    sv_health_ind: u("health").unwrap_or_default() as u8,
                    l2_p_data_flag: u("l2p").unwrap_or_default() as u8,
                    ..Default::default()
                };
                kepler!(msg);
                Some(Message::Msg1019(msg))
            }
            Constellation::QZSS => {
                let mut msg = Msg1044T {
                    qzss_satellite_id: sv.prn,
                    qzss_week_number: (u("week")? % 1024) as u16,
                    iode: f("iode")? as u8,
                    iodc: f("iodc")? as u16,
                    toc_s: toc_s as f32,
                    af0_s: eph.clock_bias,
                    af1_s_s: eph.clock_drift as _,
                    af2_s_s2: eph.clock_drift_rate as f32,
                    tgd_s: f("tgd").unwrap_or_default() as f32,
                    sv_health_ind: u("health").unwrap_or_default() as u8,
                    ..Default::default()
                };
                kepler!(msg);
                Some(Message::Msg1044(msg))
            }
            Constellation::Galileo => {
                let source = u("source").unwrap_or_default();
                let health = u("health").unwrap_or_default();
                let week = ((u("week")? - 1024) % 4096) as u16;

                if source & (1 << 1) != 0 {
                    // F/NAV
                    let mut msg = Msg1045T {
                        gal_satellite_id: sv.prn,
                        gal_week_number: week,
                        iodnav: f("iodnav")? as u16,
                        toc_s: toc_s as f32,
                        af0_s: eph.clock_bias,
                        af1_s_s: eph.clock_drift,
                        af2_s_s2: eph.clock_drift_rate as f32,
                        bgd_e1_e5a_s: f("bgdE5aE1").unwrap_or_default() as f32,
                        e5a_sig_health_ind: ((health >> 4) & 0x3) as u8,
                        e5a_data_validity_flag: ((health >> 3) & 0x1) as u8,
                        ..Default::default()
                    };
                    kepler!(msg);
                    Some(Message::Msg1045(msg))
                } else {
                    // I/NAV
                    let mut msg = Msg1046T {
                        gal_satellite_id: sv.prn,
                        gal_week_number: week,
                        iodnav: f("iodnav")? as u16,
                        toc_s: toc_s as f32,
                        af0_s: eph.clock_bias,
                        af1_s_s: eph.clock_drift,
                        af2_s_s2: eph.clock_drift_rate as f32,
                        bgd_e1_e5a_s: f("bgdE5aE1").unwrap_or_default() as f32,
                        bgd_e1_e5b_s: f("bgdE5bE1").unwrap_or_default() as f32,
                        e1_b_data_validity_flag: (health & 0x1) as u8,
                        e1_b_sig_health_ind: ((health >> 1) & 0x3) as u8,
                        e5b_data_validity_flag: ((health >> 6) & 0x1) as u8,
                        e5b_sig_health_ind: ((health >> 7) & 0x3) as u8,
                        ..Default::default()
                    };
                    kepler!(msg);
                    Some(Message::Msg1046(msg))
                }
            }
            Constellation::BeiDou => {
                let (_, toc_nanos) = toc.to_time_scale(TimeScale::BDT).to_time_of_week();
                let mut msg = Msg1042T {
                    bds_satellite_id: sv.prn,
                    bds_week_number: (u("week")? % 8192) as u16,
                    aode: f("aode")? as u8,
                    aodc: f("aodc")? as u8,
                    toc_s: (toc_nanos as f64 * 1.0E-9) as f32,
                    a0_s: eph.clock_bias,
                    a1_s_s: eph.clock_drift,
                    a2_s_s2: eph.clock_drift_rate as f32,
                    tgd1_s: f("tgd1b1b3").unwrap_or_default() as f32,
                    tgd2_s: f("tgd2b2b3").unwrap_or_default() as f32,
                    sv_health_flag: u("health").unwrap_or_default() as u8,
                    ..Default::default()
                };
                kepler!(msg);
                Some(Message::Msg1042(msg))
            }
            _ => None,
        }
    }
}

/// Signal id of each MSM signal structure
pub trait SigIdOf {
    type Id;
    fn sig_id(band: u8, attribute: char) -> Self::Id;
}

macro_rules! sig_id_of {
    ($sig:ident, $id:ident) => {
        impl SigIdOf for rtcm_rs::msg::$sig {
            type Id = $id;
            fn sig_id(band: u8, attribute: char) -> $id {
                $id::new(band, attribute)
            }
        }
    };
}

sig_id_of!(Msg1077Sig, GpsSigId);
sig_id_of!(Msg1097Sig, GalSigId);
sig_id_of!(Msg1117Sig, QzssSigId);
sig_id_of!(Msg1127Sig, BdsSigId);
