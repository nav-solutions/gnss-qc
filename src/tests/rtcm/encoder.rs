//! Test RTCM encoder: RINEX observations, ephemerides and station coordinates
//! converted to RTCM 3 frames (MSM7, legacy 1004 & 1012, 1019, 1020, 1042, 1044, 1045, 1046,
//! 1005), to stream real data through the RTCM decoder and the NTRIP clients.
use std::collections::{BTreeMap, HashMap};

use rinex::{
    navigation::Ephemeris,
    prelude::{obs::SignalObservation, Observable},
};

use rtcm_rs::{
    msg::{
        message::Message, BdsSigId, GalSigId, GloSigId, GpsSigId, Msg1004Sat, Msg1004T, Msg1005T,
        Msg1012Sat, Msg1012T, Msg1019T, Msg1020T, Msg1042T, Msg1044T, Msg1045T, Msg1046T, Msg1077T,
        Msg1087T, Msg1097T, Msg1107T, Msg1117T, Msg1127T, QzssSigId, SbasSigId,
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
    /// Frequency channel of the Glonass satellites: Glonass and SBAS signals are encoded
    /// when defined
    glonass_channels: Option<HashMap<u8, i8>>,
}

/// Glonass time of day (ms): UTC(SU) + 3 h
pub fn glonass_tod_ms(epoch: Epoch) -> u32 {
    let utc = epoch.to_time_scale(TimeScale::UTC);
    let tod_ms = (utc - utc.floor(Duration::from_days(1.0))).total_nanoseconds() / 1_000_000;
    ((tod_ms as i64 + 3 * 3_600_000).rem_euclid(86_400_000)) as u32
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
    /// Encodes the Glonass (frequency channel of each satellite: RINEX header) and
    /// SBAS signals too
    pub fn with_glonass_sbas(mut self, channels: &HashMap<SV, i8>) -> Self {
        self.glonass_channels = Some(channels.iter().map(|(sv, k)| (sv.prn, *k)).collect());
        self
    }

    /// Wavelength of a signal (m): Glonass FDMA signals depend on the frequency channel
    fn wavelength(&self, sv: SV, code: &str) -> Option<f64> {
        if sv.constellation == Constellation::Glonass
            && (code.starts_with('1') || code.starts_with('2'))
        {
            let k = *self.glonass_channels.as_ref()?.get(&sv.prn)? as f64;
            let f = if code.starts_with('1') {
                1602.0E6 + k * 562.5E3
            } else {
                1246.0E6 + k * 437.5E3
            };
            return Some(LIGHT_MS * 1.0E3 / f);
        }
        Observable::PhaseRange(format!("L{}", code))
            .to_carrier(sv.constellation)
            .ok()
            .map(|carrier| carrier.wavelength())
    }

    /// MSM satellite ID
    fn msm_id(sv: SV) -> u8 {
        if sv.constellation.is_sbas() {
            sv.prn - 19 // S20: PRN 120, MSM satellite 1
        } else {
            sv.prn
        }
    }

    /// MSM7 messages of one epoch (GPS, Galileo, BeiDou and QZSS signals, Glonass and SBAS
    /// signals with [Self::with_glonass_sbas])
    pub fn msm7(&mut self, epoch: Epoch, signals: &[SignalObservation]) -> Vec<Message> {
        // constellation > satellite > signal code ("1C")
        let mut cells: BTreeMap<Constellation, BTreeMap<u8, BTreeMap<String, Cell>>> =
            BTreeMap::new();

        for signal in signals.iter() {
            let constellation = signal.sv.constellation;
            let constellation = if constellation.is_sbas() {
                Constellation::SBAS
            } else {
                constellation
            };
            let enabled = match constellation {
                Constellation::GPS
                | Constellation::Galileo
                | Constellation::BeiDou
                | Constellation::QZSS => true,
                Constellation::Glonass | Constellation::SBAS => self.glonass_channels.is_some(),
                _ => false,
            };
            if !enabled {
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
                .entry(Self::msm_id(signal.sv))
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
            Constellation::Glonass => GloSigId::new(band, attribute).is_valid(),
            c if c.is_sbas() => SbasSigId::new(band, attribute).is_valid(),
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
            ($type:ident, $variant:ident, $sig_id:ident, $time_field:ident, $tow:expr) => {{
                let mut msg = $type::default();
                msg.reference_station_id = 1;
                msg.$time_field = $tow;
                msg.msm_multiple_message_flag = multiple as u8;

                for (prn, signals) in satellites.iter() {
                    let sv = match constellation {
                        Constellation::SBAS => {
                            use std::str::FromStr;
                            SV::from_str(&format!("S{:02}", *prn as u16 + 19)).unwrap()
                        }
                        _ => SV::new(constellation, *prn),
                    };

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
                        let wavelength = self.wavelength(sv, code)?;
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

                        let wavelength = self.wavelength(sv, code);

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

                msg
            }};
        }

        let tow = gps_tow_ms(epoch);

        Some(match constellation {
            Constellation::GPS => Message::Msg1077(build!(
                Msg1077T,
                Msm57Sat,
                Msg1077Sig,
                gps_epoch_time_ms,
                tow
            )),
            Constellation::Galileo => Message::Msg1097(build!(
                Msg1097T,
                Msm57Sat,
                Msg1097Sig,
                gal_epoch_time_ms,
                tow
            )),
            Constellation::QZSS => Message::Msg1117(build!(
                Msg1117T,
                Msm57Sat,
                Msg1117Sig,
                qzss_epoch_time_ms,
                tow
            )),
            Constellation::BeiDou => Message::Msg1127(build!(
                Msg1127T,
                Msm57Sat,
                Msg1127Sig,
                bds_epoch_time_ms,
                (tow as i64 - 14_000).rem_euclid(604_800_000) as u32
            )),
            Constellation::SBAS => Message::Msg1107(build!(
                Msg1107T,
                Msm57Sat,
                Msg1107Sig,
                gps_epoch_time_ms,
                tow
            )),
            Constellation::Glonass => {
                let mut msg = build!(
                    Msg1087T,
                    Msm57GloSat,
                    Msg1087Sig,
                    glo_epoch_time_ms,
                    glonass_tod_ms(epoch)
                );
                msg.glo_day_of_week = None;
                for sat in msg.data_segment.satellite_data.iter_mut() {
                    sat.glonass_satellite_frequency_channel_number = self
                        .glonass_channels
                        .as_ref()
                        .and_then(|channels| channels.get(&sat.satellite_id).copied());
                }
                Message::Msg1087(msg)
            }
            _ => return None,
        })
    }

    /// Legacy observations of one epoch: 1004 (GPS C1C L1C, C2W L2W),
    /// 1012 (Glonass C1C L1C, C2P L2P, with [Self::with_glonass_sbas])
    pub fn legacy(&mut self, epoch: Epoch, signals: &[SignalObservation]) -> Vec<Message> {
        // satellite > observable > value, lock loss
        let mut gps = BTreeMap::<u8, HashMap<String, (f64, bool)>>::new();
        let mut glo = BTreeMap::<u8, HashMap<String, (f64, bool)>>::new();

        for signal in signals.iter() {
            let target = match signal.sv.constellation {
                Constellation::GPS => &mut gps,
                Constellation::Glonass if self.glonass_channels.is_some() => &mut glo,
                _ => continue,
            };
            let lock_loss = signal
                .lli
                .is_some_and(|lli| lli.intersects(rinex::prelude::obs::LliFlags::LOCK_LOSS));
            target
                .entry(signal.sv.prn)
                .or_default()
                .insert(signal.observable.to_string(), (signal.value, lock_loss));
        }

        let mut messages = Vec::new();

        // (pseudo range modulus, L1 code, L2 code)
        for (constellation, satellites, modulus, l2) in [
            (Constellation::GPS, gps, 299_792.458, "2W"),
            (Constellation::Glonass, glo, 599_584.916, "2P"),
        ] {
            let mut cells = Vec::new();

            for (prn, values) in satellites.iter() {
                let sv = SV::new(constellation, *prn);
                let Some((pr1, _)) = values.get("C1C").copied() else {
                    continue;
                };

                let ambiguity = (pr1 / modulus).floor();
                let pr2 = values.get(&format!("C{}", l2)).map(|(v, _)| *v);

                // the differences are relative to the pseudo range as transmitted (0.02 m)
                let pr1 = ambiguity * modulus + ((pr1 - ambiguity * modulus) / 0.02).round() * 0.02;

                // phase (m), aligned onto its pseudo range (integer cycles), minus pr1
                let mut phase = |code: &str, pr: Option<f64>| -> Option<(f64, u8)> {
                    let (cycles, lock_loss) = values.get(&format!("L{}", code)).copied()?;
                    let wavelength = self.wavelength(sv, code)?;
                    let pr = pr.unwrap_or(pr1);
                    let key = (sv, format!("legacy-{}", code));
                    let offset = match self.phase_offsets.get(&key) {
                        Some(offset) if !lock_loss => *offset,
                        _ => {
                            let offset = ((pr - cycles * wavelength) / wavelength).round();
                            self.phase_offsets.insert(key.clone(), offset);
                            offset
                        }
                    };
                    let lock = self.locks.entry(key).or_insert(0);
                    *lock = if lock_loss { 0 } else { (*lock + 1).min(127) };
                    Some(((cycles + offset) * wavelength - pr1, *lock as u8))
                };

                let l1 = phase("1C", Some(pr1));
                let l2_phase = phase(l2, pr2);
                let cnr = |code: &str| values.get(&format!("S{}", code)).map(|(v, _)| *v as f32);

                cells.push((
                    *prn,
                    pr1 - ambiguity * modulus,
                    ambiguity * modulus,
                    l1,
                    pr2.map(|pr2| (pr2 - pr1) as f32),
                    l2_phase,
                    cnr("1C"),
                    cnr(l2),
                ));
            }

            if cells.is_empty() {
                continue;
            }

            match constellation {
                Constellation::GPS => {
                    let mut msg = Msg1004T {
                        reference_station_id: 1,
                        gps_epoch_time_ms: gps_tow_ms(epoch),
                        synchronous_gnss_msg_flag: 1,
                        ..Default::default()
                    };
                    for (prn, pr1, amb, l1, l2_diff, l2_phase, cnr1, cnr2) in cells {
                        msg.satellites.push(Msg1004Sat {
                            gps_satellite_id: prn,
                            gps_l1_code_ind: 0,
                            l1_pseudorange_m: Some(pr1),
                            l1_phase_pseudorange_diff_m: l1.map(|(d, _)| d),
                            l1_lock_time_index: l1.map(|(_, l)| l).unwrap_or_default(),
                            l1_pseudorange_amb_m: amb,
                            l1_cnr_dbhz: cnr1,
                            gps_l2_code_ind: 3,
                            l2_l1_pseudorange_diff_m: l2_diff,
                            l2_phase_l1_pseudorange_diff_m: l2_phase.map(|(d, _)| d),
                            l2_lock_time_index: l2_phase.map(|(_, l)| l).unwrap_or_default(),
                            l2_cnr_dbhz: cnr2,
                        });
                    }
                    messages.push(Message::Msg1004(msg));
                }
                _ => {
                    let channels = self.glonass_channels.clone().unwrap_or_default();
                    let mut msg = Msg1012T {
                        reference_station_id: 1,
                        glo_epoch_time_ms: glonass_tod_ms(epoch),
                        synchronous_gnss_msg_flag: 1,
                        ..Default::default()
                    };
                    for (prn, pr1, amb, l1, l2_diff, l2_phase, cnr1, cnr2) in cells {
                        msg.satellites.push(Msg1012Sat {
                            glo_satellite_id: prn,
                            glo_l1_code_ind: 0,
                            glo_satellite_freq_chan_number: channels
                                .get(&prn)
                                .copied()
                                .unwrap_or_default(),
                            l1_pseudorange_m: Some(pr1),
                            l1_phase_pseudorange_diff_m: l1.map(|(d, _)| d),
                            l1_lock_time_index: l1.map(|(_, l)| l).unwrap_or_default(),
                            l1_pseudorange_amb_m: amb,
                            l1_cnr_dbhz: cnr1,
                            glo_l2_code_ind: 1,
                            l2_l1_pseudorange_diff_m: l2_diff,
                            l2_phase_l1_pseudorange_diff_m: l2_phase.map(|(d, _)| d),
                            l2_lock_time_index: l2_phase.map(|(_, l)| l).unwrap_or_default(),
                            l2_cnr_dbhz: cnr2,
                        });
                    }
                    messages.push(Message::Msg1012(msg));
                }
            }
        }

        messages
    }

    /// Glonass radio ephemeris (1020)
    pub fn glonass_ephemeris(sv: SV, toc: Epoch, eph: &Ephemeris) -> Option<Message> {
        let f = |key: &str| eph.get_orbit_f64(key);
        let channel = match eph.orbits.get("channel")? {
            rinex::navigation::OrbitItem::I8(k) => *k,
            other => other.as_f64() as i8,
        };
        // tb: Glonass time of day (minutes, multiple of 15)
        let tb_min = (glonass_tod_ms(toc) / 60_000) as u16;
        Some(Message::Msg1020(Msg1020T {
            glo_satellite_id: sv.prn,
            glo_satellite_freq_chan_number: channel,
            tb_min,
            xn_km: f("satPosX")?,
            xn_first_deriv_km_s: f("velX")?,
            xn_second_deriv_km_s2: f("accelX")? as f32,
            yn_km: f("satPosY")?,
            yn_first_deriv_km_s: f("velY")?,
            yn_second_deriv_km_s2: f("accelY")? as f32,
            zn_km: f("satPosZ")?,
            zn_first_deriv_km_s: f("velZ")?,
            zn_second_deriv_km_s2: f("accelZ")? as f32,
            tau_n_s: -eph.clock_bias,
            gamma_n: eph.clock_drift as f32,
            en_d: f("ageOp").unwrap_or_default() as u8,
            glo_m_n4_year: 1992 + 4 * ((toc.to_gregorian_utc().0 as u16 - 1992) / 4),
            ..Default::default()
        }))
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
sig_id_of!(Msg1087Sig, GloSigId);
sig_id_of!(Msg1107Sig, SbasSigId);
