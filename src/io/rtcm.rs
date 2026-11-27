//! RTCM 3 inputs.
//!
//! [QcRtcmDecoder] converts RTCM 3 messages (decoded by [rtcm_rs]) into the data points
//! of the MISO engine ([QcInputItem]). It does not perform any I/O, so it serves RTCM files,
//! serial ports and network streams alike (see the NTRIP client, `ntrip` feature):
//!
//! | Messages | Constellation | Data points |
//! |----------|---------------|-------------|
//! | 1074 - 1077 (MSM4 - MSM7) | GPS | Signals: pseudo range, phase, doppler (MSM5/7), C/N0 |
//! | 1084 - 1087 | Glonass | idem (FDMA: frequency channel of MSM5/7, 1020 or 1012) |
//! | 1094 - 1097 | Galileo | idem |
//! | 1104 - 1107 | SBAS | idem |
//! | 1114 - 1117 | QZSS | idem |
//! | 1124 - 1127 | BeiDou | idem |
//! | 1004 (legacy L1/L2) | GPS | Signals: pseudo range, phase, C/N0, when the stream has no GPS MSM |
//! | 1012 (legacy L1/L2) | Glonass | idem, when the stream has no Glonass MSM |
//! | 1019 | GPS | Radio ephemeris |
//! | 1020 | Glonass | Radio ephemeris (position, velocity, acceleration: PZ-90, km) |
//! | 1045 (F/NAV), 1046 (I/NAV) | Galileo | Radio ephemeris |
//! | 1042 | BeiDou | Radio ephemeris |
//! | 1044 | QZSS | Radio ephemeris |
//! | 1005, 1006 | - | Reference position of the station (antenna reference point) |
//! | 1008, 1033 | - | Antenna and receiver descriptors ([QcRtcmStats]) |
//! | 1230 | Glonass | Code-phase biases of the station ([QcRtcmStats]) |
//!
//! The other messages (SSR, network RTK..) are counted in [QcRtcmStats] but not interpreted.
//!
//! ## Legacy and MSM observations
//! Many reference stations stream the same observations twice: legacy messages (1004, 1012)
//! for older rovers, and MSM. Once MSM observations of a constellation were decoded, the legacy
//! messages of that constellation are skipped ([QcRtcmStats::redundant]): the MSM carry more
//! signals, at a better resolution.
//!
//! ## Glonass
//! Glonass time tags (time of day, Moscow time) are converted to [TimeScale::GPST], like the
//! other constellations. Glonass signals are frequency multiplexed (FDMA): the phase of a
//! satellite is converted to cycles with the wavelength of its frequency channel, learned from
//! MSM5/MSM7 (extended satellite information), 1020 or 1012. Until the channel of a satellite is
//! known, only its pseudo ranges and C/N0 are delivered.
//!
//! ## Time tags
//! MSM messages only carry the time of week (or day) of the observations. The week is
//! resolved against a reference [Epoch]: the system time by default (real-time streams),
//! or the approximate date of recorded data ([QcRtcmDecoder::with_reference_epoch]).
//! Then the reference follows the stream. All signals are time tagged in [TimeScale::GPST],
//! so the observations of all constellations of one epoch share the same time tag.
//!
//! Radio ephemerides and station coordinates are time tagged with their *reception* time
//! (time tag of the last observation, or the reference): the sources of the engine must remain
//! time ordered, and the Time of Clock of an ephemeris may be in the future of the stream.
//! In real-time, the ephemerides and coordinates received before the first observation are
//! held until that observation (then tagged with its epoch), so that the stream remains time
//! ordered: the system time is ahead of the observations (latency of the stream). A stream
//! without observations (ephemerides only) releases them after 10 s, tagged with the system time.
//!
//! ## Signals
//! MSM signals are expressed as RINEX observations, in RINEX units:
//! `C1C` (pseudo range, m), `L1C` (phase, cycles), `D1C` (doppler, Hz), `S1C` (C/N0, dB-Hz).
//! MSM phase ranges are aligned with the pseudo ranges by the receiver (RTCM 3.3 standard),
//! the lock time indicators are converted to loss of lock flags.
use std::{
    collections::{BTreeMap, HashMap},
    str::FromStr,
};

use gnss_rs::prelude::{Constellation, SV};
use hifitime::{Duration, Epoch, TimeScale};

use rinex::{
    navigation::{Ephemeris, NavMessageType, OrbitItem},
    prelude::{
        obs::{EpochFlag, LliFlags, SignalObservation, SNR},
        Observable,
    },
};

use rtcm_rs::{msg::message::Message, MessageFrame};

#[cfg(feature = "log")]
use log::{debug, warn};

use crate::io::source::{
    QcEphemeris, QcInputItem, QcPayload, QcPoll, QcSource, QcSourceDescriptor, QcSourceError,
};

/// Speed of light, in meters per millisecond
const LIGHT_MS: f64 = 299_792.458;

/// One week, in milliseconds
const WEEK_MS: i64 = 604_800_000;

/// BeiDou time (BDT) is 14 s behind GPST
const BDT_GPST_OFFSET_MS: i64 = 14_000;

/// Galileo week number (DF289) counts from GPS week 1024
const GST_GPS_WEEK_OFFSET: u32 = 1024;

/// Glonass time is UTC(SU) + 3 hours
const GLONASS_UTC_OFFSET_MS: i64 = 3 * 3_600_000;

/// One day, in milliseconds
const DAY_MS: i64 = 86_400_000;

/// Context data held before the first observation of a real-time stream, at most
const PENDING_CONTEXT: std::time::Duration = std::time::Duration::from_secs(10);

/// Statistics of a decoded RTCM stream
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QcRtcmStats {
    /// Total number of messages
    pub messages: usize,

    /// Number of observation epochs (MSM messages)
    pub msm: usize,

    /// Number of signal observations
    pub signals: usize,

    /// Number of radio ephemerides
    pub ephemerides: usize,

    /// Number of station coordinates messages (1005, 1006)
    pub stations: usize,

    /// Number of legacy observation messages decoded (1004, 1012)
    pub legacy: usize,

    /// Legacy observation messages skipped: MSM observations of the same constellation
    /// are streamed too
    pub redundant: usize,

    /// Antenna descriptor and serial number (1008, 1033)
    pub antenna: Option<String>,

    /// Receiver type, firmware and serial number (1033)
    pub receiver: Option<String>,

    /// Glonass code-phase biases of the station (1230), per signal (`1C`, `2P`..), in meters
    pub glonass_code_phase_biases: BTreeMap<String, f64>,

    /// Messages that were not interpreted, by message number
    pub ignored: BTreeMap<u16, usize>,
}

/// Satellite data of one MSM message
struct MsmSatellite {
    id: u8,
    /// Rough range (ms)
    rough_ms: Option<f64>,
    /// Rough phase range rate (m/s), MSM5 & MSM7
    rough_rate_m_s: Option<f64>,
    /// Glonass frequency channel, MSM5 & MSM7
    glonass_channel: Option<i8>,
}

/// Signal data of one MSM message
struct MsmSignal {
    satellite: u8,
    band: u8,
    attribute: char,
    /// Fine pseudo range (ms)
    pseudo_range_ms: Option<f64>,
    /// Fine phase range (ms)
    phase_range_ms: Option<f64>,
    /// Lock time indicator
    lock: u16,
    /// Half cycle ambiguity
    half_cycle: bool,
    /// C/N0 (dB-Hz)
    cnr_dbhz: Option<f64>,
    /// Fine phase range rate (m/s), MSM5 & MSM7
    rate_m_s: Option<f64>,
}

/// One decoded MSM message
struct Msm {
    constellation: Constellation,
    /// Legacy observations (1004, 1012)
    legacy: bool,
    /// Time of week (ms, GPST)
    gps_tow_ms: i64,
    satellites: Vec<MsmSatellite>,
    signals: Vec<MsmSignal>,
}

/// Uniform access to the RTCM data fields (plain or optional values)
trait Field {
    fn value(&self) -> Option<f64>;
}

impl Field for f64 {
    fn value(&self) -> Option<f64> {
        Some(*self)
    }
}

impl Field for f32 {
    fn value(&self) -> Option<f64> {
        Some(*self as f64)
    }
}

impl Field for u8 {
    fn value(&self) -> Option<f64> {
        Some(*self as f64)
    }
}

impl Field for u16 {
    fn value(&self) -> Option<f64> {
        Some(*self as f64)
    }
}

impl<T: Field> Field for Option<T> {
    fn value(&self) -> Option<f64> {
        self.as_ref().and_then(|v| v.value())
    }
}

impl Field for i16 {
    fn value(&self) -> Option<f64> {
        Some(*self as f64)
    }
}

/// MSM4 & MSM6: no phase range rates
macro_rules! msm46 {
    ($msg:expr, $constellation:expr, $tow:expr, $pr:ident, $cp:ident, $lock:ident, $cnr:ident) => {{
        let data = &$msg.data_segment;
        Msm {
            constellation: $constellation,
            legacy: false,
            gps_tow_ms: $tow,
            satellites: data
                .satellite_data
                .iter()
                .map(|sat| MsmSatellite {
                    id: sat.satellite_id,
                    rough_ms: rough_ms(
                        sat.gnss_satellite_rough_range_integer_ms.value(),
                        sat.gnss_satellite_rough_range_mod1ms_ms.value(),
                    ),
                    rough_rate_m_s: None,
                    glonass_channel: None,
                })
                .collect(),
            signals: data
                .signal_data
                .iter()
                .map(|sig| MsmSignal {
                    satellite: sig.satellite_id,
                    band: sig.signal_id.band(),
                    attribute: sig.signal_id.attribute(),
                    pseudo_range_ms: sig.$pr.value(),
                    phase_range_ms: sig.$cp.value(),
                    lock: sig.$lock as u16,
                    half_cycle: sig.half_cycle_ambiguity_ind != 0,
                    cnr_dbhz: sig.$cnr.value(),
                    rate_m_s: None,
                })
                .collect(),
        }
    }};
}

/// MSM5 & MSM7: with phase range rates
macro_rules! msm57 {
    ($msg:expr, $constellation:expr, $tow:expr, $pr:ident, $cp:ident, $lock:ident, $cnr:ident) => {{
        let data = &$msg.data_segment;
        Msm {
            constellation: $constellation,
            legacy: false,
            gps_tow_ms: $tow,
            satellites: data
                .satellite_data
                .iter()
                .map(|sat| MsmSatellite {
                    id: sat.satellite_id,
                    rough_ms: rough_ms(
                        sat.gnss_satellite_rough_range_integer_ms.value(),
                        sat.gnss_satellite_rough_range_mod1ms_ms.value(),
                    ),
                    rough_rate_m_s: sat.gnss_satellite_rough_phaserange_rates_m_s.value(),
                    glonass_channel: None,
                })
                .collect(),
            signals: data
                .signal_data
                .iter()
                .map(|sig| MsmSignal {
                    satellite: sig.satellite_id,
                    band: sig.signal_id.band(),
                    attribute: sig.signal_id.attribute(),
                    pseudo_range_ms: sig.$pr.value(),
                    phase_range_ms: sig.$cp.value(),
                    lock: sig.$lock as u16,
                    half_cycle: sig.half_cycle_ambiguity_ind != 0,
                    cnr_dbhz: sig.$cnr.value(),
                    rate_m_s: sig.gnss_signal_fine_phaserange_rate_m_s.value(),
                })
                .collect(),
        }
    }};
}

/// Rough range (ms) of one satellite
fn rough_ms(integer_ms: Option<f64>, mod1ms_ms: Option<f64>) -> Option<f64> {
    Some(integer_ms? + mod1ms_ms?)
}

/// Unwraps a time of week (ms) in the week closest to the reference
fn resolve_tow(reference: Epoch, tow_ms: i64) -> Epoch {
    let (week, nanos) = reference.to_time_scale(TimeScale::GPST).to_time_of_week();
    let reference_ms = (nanos / 1_000_000) as i64;

    let mut week = week as i64;
    let tow_ms = tow_ms.rem_euclid(WEEK_MS);

    if tow_ms - reference_ms > WEEK_MS / 2 {
        week -= 1;
    } else if reference_ms - tow_ms > WEEK_MS / 2 {
        week += 1;
    }

    Epoch::from_time_of_week(
        week.max(0) as u32,
        tow_ms as u64 * 1_000_000,
        TimeScale::GPST,
    )
}

/// Unwraps a truncated week counter (`modulo` weeks) in the cycle closest to the reference week
fn resolve_week(reference_week: u32, week: u32, modulo: u32) -> u32 {
    let base = reference_week - reference_week % modulo + week % modulo;
    [base.saturating_sub(modulo), base, base + modulo]
        .into_iter()
        .min_by_key(|candidate| (*candidate as i64 - reference_week as i64).abs())
        .unwrap_or(base)
}

/// Glonass time (UTC(SU) + 3 h) of day (ms) to the [TimeScale::GPST] epoch closest
/// to the reference
fn glonass_epoch(reference: Epoch, glonass_tod_ms: i64) -> Epoch {
    let reference = reference.to_time_scale(TimeScale::UTC);
    let day = Duration::from_days(1.0);

    let utc_tod_ms = (glonass_tod_ms - GLONASS_UTC_OFFSET_MS).rem_euclid(DAY_MS);
    let candidate = reference.floor(day) + Duration::from_milliseconds(utc_tod_ms as f64);

    [candidate - day, candidate, candidate + day]
        .into_iter()
        .min_by_key(|t| (*t - reference).abs().total_nanoseconds())
        .unwrap_or(candidate)
        .to_time_scale(TimeScale::GPST)
}

/// Glonass time (UTC(SU) + 3 h) of day (ms) to the GPS time of week (ms)
fn glonass_tow_ms(reference: Epoch, glonass_tod_ms: i64) -> i64 {
    let (_, nanos) = glonass_epoch(reference, glonass_tod_ms).to_time_of_week();
    (nanos as f64 / 1.0E6).round() as i64
}

/// Wavelength (m) of a signal: Glonass FDMA signals (bands 1 and 2) need the frequency
/// channel of the satellite
fn wavelength_m(
    constellation: Constellation,
    band: u8,
    attribute: char,
    glonass_channel: Option<i8>,
) -> Option<f64> {
    if constellation == Constellation::Glonass && (band == 1 || band == 2) {
        let k = glonass_channel? as f64;
        let frequency_hz = if band == 1 {
            1602.0E6 + k * 562.5E3
        } else {
            1246.0E6 + k * 437.5E3
        };
        return Some(LIGHT_MS * 1.0E3 / frequency_hz);
    }

    Observable::PhaseRange(format!("L{}{}", band, attribute))
        .to_carrier(constellation)
        .ok()
        .map(|carrier| carrier.wavelength())
}

/// One satellite of a legacy observation message (1004, 1012), as MSM data
#[allow(clippy::too_many_arguments)]
fn legacy_signals(
    satellite: u8,
    l1_attribute: char,
    l2_attribute: Option<char>,
    pseudo_range_m: Option<f64>,
    l1_phase_diff_m: Option<f64>,
    l1_lock: u8,
    l1_cnr: Option<f64>,
    l2_pseudo_range_diff_m: Option<f64>,
    l2_phase_diff_m: Option<f64>,
    l2_lock: u8,
    l2_cnr: Option<f64>,
) -> Vec<MsmSignal> {
    let Some(pr1_m) = pseudo_range_m else {
        return Vec::new();
    };

    let ms = |m: f64| m / LIGHT_MS;

    let mut signals = vec![MsmSignal {
        satellite,
        band: 1,
        attribute: l1_attribute,
        pseudo_range_ms: Some(ms(pr1_m)),
        phase_range_ms: l1_phase_diff_m.map(|d| ms(pr1_m + d)),
        lock: l1_lock as u16,
        half_cycle: false,
        cnr_dbhz: l1_cnr,
        rate_m_s: None,
    }];

    if let Some(l2_attribute) = l2_attribute {
        let pseudo_range_ms = l2_pseudo_range_diff_m.map(|d| ms(pr1_m + d));
        let phase_range_ms = l2_phase_diff_m.map(|d| ms(pr1_m + d));
        if pseudo_range_ms.is_some() || phase_range_ms.is_some() {
            signals.push(MsmSignal {
                satellite,
                band: 2,
                attribute: l2_attribute,
                pseudo_range_ms,
                phase_range_ms,
                lock: l2_lock as u16,
                half_cycle: false,
                cnr_dbhz: l2_cnr,
                rate_m_s: None,
            });
        }
    }

    signals
}

/// Legacy satellites: rough range 0, the signals carry the full ranges
fn legacy_satellites(ids: impl Iterator<Item = u8>) -> Vec<MsmSatellite> {
    ids.map(|id| MsmSatellite {
        id,
        rough_ms: Some(0.0),
        rough_rate_m_s: None,
        glonass_channel: None,
    })
    .collect()
}

/// GPS & QZSS URA index to accuracy (m)
fn ura_meters(index: u8) -> f64 {
    const URA: [f64; 15] = [
        2.4, 3.4, 4.85, 6.85, 9.65, 13.65, 24.0, 48.0, 96.0, 192.0, 384.0, 768.0, 1536.0, 3072.0,
        6144.0,
    ];
    URA.get(index as usize).copied().unwrap_or(-1.0)
}

/// Galileo SISA index to accuracy (m)
fn sisa_meters(index: u8) -> f64 {
    let index = index as f64;
    match index as u8 {
        0..=49 => index * 0.01,
        50..=74 => 0.5 + (index - 50.0) * 0.02,
        75..=99 => 1.0 + (index - 75.0) * 0.04,
        100..=125 => 2.0 + (index - 100.0) * 0.16,
        _ => -1.0,
    }
}

/// Helper to build the orbital fields of an [Ephemeris],
/// with the RINEX names, types and units (radians)
struct OrbitBuilder {
    constellation: Constellation,
    ephemeris: Ephemeris,
}

impl OrbitBuilder {
    fn new(constellation: Constellation, af0: f64, af1: f64, af2: f64) -> Self {
        Self {
            constellation,
            ephemeris: Ephemeris {
                clock_bias: af0,
                clock_drift: af1,
                clock_drift_rate: af2,
                orbits: HashMap::with_capacity(32),
            },
        }
    }

    /// Plain value
    fn f64(mut self, key: &str, value: f64) -> Self {
        self.ephemeris
            .orbits
            .insert(key.to_string(), OrbitItem::F64(value));
        self
    }

    /// Semi circles (RTCM) to radians (RINEX)
    fn semi_circles(self, key: &str, value: f64) -> Self {
        self.f64(key, value * std::f64::consts::PI)
    }

    fn u32(mut self, key: &str, value: u32) -> Self {
        self.ephemeris
            .orbits
            .insert(key.to_string(), OrbitItem::U32(value));
        self
    }

    /// Bit flags: interpreted as the RINEX parser does
    fn flags(mut self, key: &str, value: u32) -> Self {
        match OrbitItem::new(
            key,
            "flag",
            &format!("{}.0", value),
            &NavMessageType::LNAV,
            self.constellation,
        ) {
            Ok(item) => {
                self.ephemeris.orbits.insert(key.to_string(), item);
            }
            Err(_) => {
                #[cfg(feature = "log")]
                warn!(
                    "rtcm: invalid {} {} flags: {}",
                    self.constellation, key, value
                );
            }
        }
        self
    }

    /// Kepler parameters shared by all constellations
    #[allow(clippy::too_many_arguments)]
    fn kepler(
        self,
        crs: f64,
        delta_n_sc: f64,
        m0_sc: f64,
        cuc: f64,
        e: f64,
        cus: f64,
        sqrta: f64,
        toe_s: f64,
        cic: f64,
        omega0_sc: f64,
        cis: f64,
        i0_sc: f64,
        crc: f64,
        omega_sc: f64,
        omega_dot_sc: f64,
        idot_sc: f64,
    ) -> Self {
        self.f64("crs", crs)
            .semi_circles("deltaN", delta_n_sc)
            .semi_circles("m0", m0_sc)
            .f64("cuc", cuc)
            .f64("e", e)
            .f64("cus", cus)
            .f64("sqrta", sqrta)
            .f64("toe", toe_s)
            .f64("cic", cic)
            .semi_circles("omega0", omega0_sc)
            .f64("cis", cis)
            .semi_circles("i0", i0_sc)
            .f64("crc", crc)
            .semi_circles("omega", omega_sc)
            .semi_circles("omegaDot", omega_dot_sc)
            .semi_circles("idot", idot_sc)
    }
}

/// RTCM 3 decoder, see the [module](crate::io::rtcm) documentation.
#[derive(Debug, Clone, Default)]
pub struct QcRtcmDecoder {
    /// Reference [Epoch] to resolve the weeks
    reference: Option<Epoch>,
    /// Lock time indicators of each signal, to detect loss of lock
    locks: HashMap<(SV, u8, char), u16>,
    /// Frequency channel of the Glonass satellites
    glonass_channels: HashMap<u8, i8>,
    /// Constellations streamed as MSM (legacy observations of these are redundant)
    msm_constellations: std::collections::HashSet<Constellation>,
    /// Observations delivered for the latest epoch (no duplicate when legacy and MSM
    /// messages describe the same epoch)
    delivered: (Option<Epoch>, std::collections::HashSet<(SV, String)>),
    /// Context data received before the first observation (real-time)
    pending: Vec<QcInputItem>,
    /// Reception of the oldest pending context data
    pending_since: Option<std::time::Instant>,
    /// Statistics
    stats: QcRtcmStats,
}

impl QcRtcmDecoder {
    /// New [QcRtcmDecoder], whose time references follow the system time (real-time streams)
    pub fn new() -> Self {
        Self::default()
    }

    /// Defines the approximate [Epoch] of the stream, to resolve the weeks
    /// (recorded streams). Default: system time.
    pub fn with_reference_epoch(mut self, epoch: Epoch) -> Self {
        self.reference = Some(epoch);
        self
    }

    /// Statistics of the decoded stream
    pub fn stats(&self) -> &QcRtcmStats {
        &self.stats
    }

    /// Time reference of the stream: last observation epoch, or the reference epoch,
    /// or the system time.
    fn reference(&self) -> Epoch {
        self.reference.unwrap_or_else(|| {
            Epoch::now()
                .unwrap_or_else(|_| Epoch::from_gregorian_utc_at_midnight(2000, 1, 1))
                .to_time_scale(TimeScale::GPST)
        })
    }

    /// Decodes all complete frames of `bytes` (frames are detected by their preamble and CRC),
    /// stores the data points in `items`, and returns the number of bytes consumed.
    /// A truncated frame is left in place (not consumed), so the remaining bytes may be completed
    /// by the next read of a byte stream.
    pub fn decode_bytes(&mut self, bytes: &[u8], items: &mut Vec<QcInputItem>) -> usize {
        let mut consumed = 0;

        while consumed < bytes.len() {
            match bytes[consumed..].iter().position(|b| *b == 0xd3) {
                Some(offset) => consumed += offset,
                None => return bytes.len(),
            }

            match MessageFrame::new(&bytes[consumed..]) {
                Ok(frame) => {
                    self.decode(&frame.get_message(), items);
                    consumed += frame.frame_len();
                }
                Err(rtcm_rs::rtcm_error::RtcmError::Incomplete) => return consumed,
                Err(_) => consumed += 1,
            }
        }

        consumed
    }

    /// Decodes one RTCM [Message] and stores the resulting data points in `items`.
    pub fn decode(&mut self, message: &Message, items: &mut Vec<QcInputItem>) {
        self.stats.messages += 1;

        // ephemerides only stream: releases the context data held so far
        self.release_pending(None, items);

        let msm = match message {
            Message::Msg1074(m) => msm46!(
                m,
                Constellation::GPS,
                m.gps_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ms,
                gnss_signal_fine_phaserange_ms,
                gnss_phaserange_lock_time_ind,
                gnss_signal_cnr_dbhz
            ),
            Message::Msg1075(m) => msm57!(
                m,
                Constellation::GPS,
                m.gps_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ms,
                gnss_signal_fine_phaserange_ms,
                gnss_phaserange_lock_time_ind,
                gnss_signal_cnr_dbhz
            ),
            Message::Msg1076(m) => msm46!(
                m,
                Constellation::GPS,
                m.gps_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ext_ms,
                gnss_signal_fine_phaserange_ext_ms,
                gnss_phaserange_lock_time_ext_ind,
                gnss_signal_cnr_ext_dbhz
            ),
            Message::Msg1077(m) => msm57!(
                m,
                Constellation::GPS,
                m.gps_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ext_ms,
                gnss_signal_fine_phaserange_ext_ms,
                gnss_phaserange_lock_time_ext_ind,
                gnss_signal_cnr_ext_dbhz
            ),
            Message::Msg1094(m) => msm46!(
                m,
                Constellation::Galileo,
                m.gal_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ms,
                gnss_signal_fine_phaserange_ms,
                gnss_phaserange_lock_time_ind,
                gnss_signal_cnr_dbhz
            ),
            Message::Msg1095(m) => msm57!(
                m,
                Constellation::Galileo,
                m.gal_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ms,
                gnss_signal_fine_phaserange_ms,
                gnss_phaserange_lock_time_ind,
                gnss_signal_cnr_dbhz
            ),
            Message::Msg1096(m) => msm46!(
                m,
                Constellation::Galileo,
                m.gal_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ext_ms,
                gnss_signal_fine_phaserange_ext_ms,
                gnss_phaserange_lock_time_ext_ind,
                gnss_signal_cnr_ext_dbhz
            ),
            Message::Msg1097(m) => msm57!(
                m,
                Constellation::Galileo,
                m.gal_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ext_ms,
                gnss_signal_fine_phaserange_ext_ms,
                gnss_phaserange_lock_time_ext_ind,
                gnss_signal_cnr_ext_dbhz
            ),
            Message::Msg1114(m) => msm46!(
                m,
                Constellation::QZSS,
                m.qzss_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ms,
                gnss_signal_fine_phaserange_ms,
                gnss_phaserange_lock_time_ind,
                gnss_signal_cnr_dbhz
            ),
            Message::Msg1115(m) => msm57!(
                m,
                Constellation::QZSS,
                m.qzss_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ms,
                gnss_signal_fine_phaserange_ms,
                gnss_phaserange_lock_time_ind,
                gnss_signal_cnr_dbhz
            ),
            Message::Msg1116(m) => msm46!(
                m,
                Constellation::QZSS,
                m.qzss_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ext_ms,
                gnss_signal_fine_phaserange_ext_ms,
                gnss_phaserange_lock_time_ext_ind,
                gnss_signal_cnr_ext_dbhz
            ),
            Message::Msg1117(m) => msm57!(
                m,
                Constellation::QZSS,
                m.qzss_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ext_ms,
                gnss_signal_fine_phaserange_ext_ms,
                gnss_phaserange_lock_time_ext_ind,
                gnss_signal_cnr_ext_dbhz
            ),
            Message::Msg1124(m) => msm46!(
                m,
                Constellation::BeiDou,
                m.bds_epoch_time_ms as i64 + BDT_GPST_OFFSET_MS,
                gnss_signal_fine_pseudorange_ms,
                gnss_signal_fine_phaserange_ms,
                gnss_phaserange_lock_time_ind,
                gnss_signal_cnr_dbhz
            ),
            Message::Msg1125(m) => msm57!(
                m,
                Constellation::BeiDou,
                m.bds_epoch_time_ms as i64 + BDT_GPST_OFFSET_MS,
                gnss_signal_fine_pseudorange_ms,
                gnss_signal_fine_phaserange_ms,
                gnss_phaserange_lock_time_ind,
                gnss_signal_cnr_dbhz
            ),
            Message::Msg1126(m) => msm46!(
                m,
                Constellation::BeiDou,
                m.bds_epoch_time_ms as i64 + BDT_GPST_OFFSET_MS,
                gnss_signal_fine_pseudorange_ext_ms,
                gnss_signal_fine_phaserange_ext_ms,
                gnss_phaserange_lock_time_ext_ind,
                gnss_signal_cnr_ext_dbhz
            ),
            Message::Msg1127(m) => msm57!(
                m,
                Constellation::BeiDou,
                m.bds_epoch_time_ms as i64 + BDT_GPST_OFFSET_MS,
                gnss_signal_fine_pseudorange_ext_ms,
                gnss_signal_fine_phaserange_ext_ms,
                gnss_phaserange_lock_time_ext_ind,
                gnss_signal_cnr_ext_dbhz
            ),
            Message::Msg1005(m) => {
                self.station(
                    (
                        m.antenna_ref_point_ecef_x_m,
                        m.antenna_ref_point_ecef_y_m,
                        m.antenna_ref_point_ecef_z_m,
                    ),
                    items,
                );
                return;
            }
            Message::Msg1006(m) => {
                self.station(
                    (
                        m.antenna_ref_point_ecef_x_m,
                        m.antenna_ref_point_ecef_y_m,
                        m.antenna_ref_point_ecef_z_m,
                    ),
                    items,
                );
                return;
            }
            Message::Msg1019(m) => {
                let reference_week = self.reference().to_time_of_week().0;
                let week = resolve_week(reference_week, m.gps_week_number as u32, 1024);
                let sv = SV::new(Constellation::GPS, m.gps_satellite_id);

                let ephemeris = OrbitBuilder::new(
                    Constellation::GPS,
                    m.af0_s,
                    m.af1_s_s as f64,
                    m.af2_s_s2 as f64,
                )
                .f64("iode", m.iode as f64)
                .kepler(
                    m.crs_m as f64,
                    m.delta_n_sc_s as f64,
                    m.m0_sc,
                    m.cuc_rad as f64,
                    m.eccentricity,
                    m.cus_rad as f64,
                    m.sqrt_a_sqrt_m,
                    m.toe_s as f64,
                    m.cic_rad as f64,
                    m.omega0_sc,
                    m.cis_rad as f64,
                    m.i0_sc,
                    m.crc_m as f64,
                    m.omega_sc,
                    m.omegadot_sc_s,
                    m.idot_sc_s,
                )
                .f64("l2Codes", m.code_on_l2_ind as f64)
                .u32("week", week)
                .flags("l2p", m.l2_p_data_flag as u32)
                .f64("accuracy", ura_meters(m.ura_index))
                .flags("health", m.sv_health_ind as u32)
                .f64("tgd", m.tgd_s as f64)
                .f64("iodc", m.iodc as f64)
                .f64("fitInt", if m.fit_interval_ind == 0 { 4.0 } else { 6.0 });

                let toc = Epoch::from_time_of_week(
                    week,
                    (m.toc_s as f64 * 1.0E9).round() as u64,
                    TimeScale::GPST,
                );

                self.ephemeris(sv, toc, ephemeris.ephemeris, items);
                return;
            }
            Message::Msg1044(m) => {
                let reference_week = self.reference().to_time_of_week().0;
                let week = resolve_week(reference_week, m.qzss_week_number as u32, 1024);
                let sv = SV::new(Constellation::QZSS, m.qzss_satellite_id);

                let ephemeris = OrbitBuilder::new(
                    Constellation::QZSS,
                    m.af0_s,
                    m.af1_s_s as f64,
                    m.af2_s_s2 as f64,
                )
                .f64("iode", m.iode as f64)
                .kepler(
                    m.crs_m as f64,
                    m.delta_n_sc_s as f64,
                    m.m0_sc,
                    m.cuc_rad as f64,
                    m.eccentricity,
                    m.cus_rad as f64,
                    m.sqrt_a_sqrt_m,
                    m.toe_s as f64,
                    m.cic_rad as f64,
                    m.omega0_sc,
                    m.cis_rad as f64,
                    m.i0_sc,
                    m.crc_m as f64,
                    m.omega_sc,
                    m.omegadot_sc_s,
                    m.idot_sc_s,
                )
                .f64("l2Codes", m.code_on_l2_ind as f64)
                .u32("week", week)
                .flags("l2p", 0)
                .f64("accuracy", ura_meters(m.ura_index))
                .flags("health", m.sv_health_ind as u32)
                .f64("tgd", m.tgd_s as f64)
                .f64("iodc", m.iodc as f64)
                .f64("fitInt", if m.fit_interval_ind == 0 { 2.0 } else { 4.0 });

                let toc = Epoch::from_time_of_week(
                    week,
                    (m.toc_s as f64 * 1.0E9).round() as u64,
                    TimeScale::GPST,
                );

                self.ephemeris(sv, toc, ephemeris.ephemeris, items);
                return;
            }
            Message::Msg1045(m) => {
                // F/NAV: E5a & E1 clock corrections
                let reference_week = self.reference().to_time_of_week().0;
                let week = resolve_week(
                    reference_week,
                    m.gal_week_number as u32 + GST_GPS_WEEK_OFFSET,
                    4096,
                );
                let sv = SV::new(Constellation::Galileo, m.gal_satellite_id);

                // E5a health (bits 4-5) & data validity (bit 3)
                let health =
                    ((m.e5a_sig_health_ind as u32) << 4) | ((m.e5a_data_validity_flag as u32) << 3);

                let ephemeris = OrbitBuilder::new(
                    Constellation::Galileo,
                    m.af0_s,
                    m.af1_s_s,
                    m.af2_s_s2 as f64,
                )
                .f64("iodnav", m.iodnav as f64)
                .kepler(
                    m.crs_m as f64,
                    m.delta_n_sc_s as f64,
                    m.m0_sc,
                    m.cuc_rad as f64,
                    m.eccentricity,
                    m.cus_rad as f64,
                    m.sqrt_a_sqrt_m,
                    m.toe_s as f64,
                    m.cic_rad as f64,
                    m.omega0_sc,
                    m.cis_rad as f64,
                    m.i0_sc,
                    m.crc_m as f64,
                    m.omega_sc,
                    m.omegadot_sc_s,
                    m.idot_sc_s as f64,
                )
                // F/NAV E5a-I (bit 1), af0-af2/Toc/SISA for E5a,E1 (bit 8)
                .flags("source", (1 << 1) | (1 << 8))
                .u32("week", week)
                .f64("sisa", sisa_meters(m.sisa_e1_e5a_index))
                .flags("health", health)
                .f64("bgdE5aE1", m.bgd_e1_e5a_s as f64)
                .f64("bgdE5bE1", 0.0);

                let toc = Epoch::from_time_of_week(
                    week,
                    (m.toc_s as f64 * 1.0E9).round() as u64,
                    TimeScale::GPST,
                );

                self.ephemeris(sv, toc, ephemeris.ephemeris, items);
                return;
            }
            Message::Msg1046(m) => {
                // I/NAV: E5b & E1 clock corrections
                let reference_week = self.reference().to_time_of_week().0;
                let week = resolve_week(
                    reference_week,
                    m.gal_week_number as u32 + GST_GPS_WEEK_OFFSET,
                    4096,
                );
                let sv = SV::new(Constellation::Galileo, m.gal_satellite_id);

                // E1-B data validity (bit 0) & health (bits 1-2),
                // E5b data validity (bit 6) & health (bits 7-8)
                let health = (m.e1_b_data_validity_flag as u32)
                    | ((m.e1_b_sig_health_ind as u32) << 1)
                    | ((m.e5b_data_validity_flag as u32) << 6)
                    | ((m.e5b_sig_health_ind as u32) << 7);

                let ephemeris = OrbitBuilder::new(
                    Constellation::Galileo,
                    m.af0_s,
                    m.af1_s_s,
                    m.af2_s_s2 as f64,
                )
                .f64("iodnav", m.iodnav as f64)
                .kepler(
                    m.crs_m as f64,
                    m.delta_n_sc_s as f64,
                    m.m0_sc,
                    m.cuc_rad as f64,
                    m.eccentricity,
                    m.cus_rad as f64,
                    m.sqrt_a_sqrt_m,
                    m.toe_s as f64,
                    m.cic_rad as f64,
                    m.omega0_sc,
                    m.cis_rad as f64,
                    m.i0_sc,
                    m.crc_m as f64,
                    m.omega_sc,
                    m.omegadot_sc_s,
                    m.idot_sc_s as f64,
                )
                // I/NAV E1-B (bit 0) & E5b-I (bit 2), af0-af2/Toc/SISA for E5b,E1 (bit 9)
                .flags("source", 1 | (1 << 2) | (1 << 9))
                .u32("week", week)
                .f64("sisa", sisa_meters(m.sisa_e1_e5b_index))
                .flags("health", health)
                .f64("bgdE5aE1", m.bgd_e1_e5a_s as f64)
                .f64("bgdE5bE1", m.bgd_e1_e5b_s as f64);

                let toc = Epoch::from_time_of_week(
                    week,
                    (m.toc_s as f64 * 1.0E9).round() as u64,
                    TimeScale::GPST,
                );

                self.ephemeris(sv, toc, ephemeris.ephemeris, items);
                return;
            }
            Message::Msg1042(m) => {
                // BDT weeks (13 bits)
                let reference_week = self
                    .reference()
                    .to_time_scale(TimeScale::BDT)
                    .to_time_of_week()
                    .0;
                let week = resolve_week(reference_week, m.bds_week_number as u32, 8192);
                let sv = SV::new(Constellation::BeiDou, m.bds_satellite_id);

                let ephemeris =
                    OrbitBuilder::new(Constellation::BeiDou, m.a0_s, m.a1_s_s, m.a2_s_s2 as f64)
                        .f64("aode", m.aode as f64)
                        .kepler(
                            m.crs_m as f64,
                            m.delta_n_sc_s as f64,
                            m.m0_sc,
                            m.cuc_rad as f64,
                            m.eccentricity,
                            m.cus_rad as f64,
                            m.sqrt_a_sqrt_m,
                            m.toe_s as f64,
                            m.cic_rad as f64,
                            m.omega0_sc,
                            m.cis_rad as f64,
                            m.i0_sc,
                            m.crc_m as f64,
                            m.omega_sc,
                            m.omegadot_sc_s,
                            m.idot_sc_s,
                        )
                        .u32("week", week)
                        .f64("accuracy", ura_meters(m.ura_index))
                        .flags("health", m.sv_health_flag as u32)
                        .f64("tgd1b1b3", m.tgd1_s as f64)
                        .f64("tgd2b2b3", m.tgd2_s as f64)
                        .f64("aodc", m.aodc as f64);

                let toc = Epoch::from_time_of_week(
                    week,
                    (m.toc_s as f64 * 1.0E9).round() as u64,
                    TimeScale::BDT,
                );

                self.ephemeris(sv, toc, ephemeris.ephemeris, items);
                return;
            }
            Message::Msg1084(m) => msm46!(
                m,
                Constellation::Glonass,
                glonass_tow_ms(self.reference(), m.glo_epoch_time_ms as i64),
                gnss_signal_fine_pseudorange_ms,
                gnss_signal_fine_phaserange_ms,
                gnss_phaserange_lock_time_ind,
                gnss_signal_cnr_dbhz
            ),
            Message::Msg1085(m) => {
                let mut msm = msm57!(
                    m,
                    Constellation::Glonass,
                    glonass_tow_ms(self.reference(), m.glo_epoch_time_ms as i64),
                    gnss_signal_fine_pseudorange_ms,
                    gnss_signal_fine_phaserange_ms,
                    gnss_phaserange_lock_time_ind,
                    gnss_signal_cnr_dbhz
                );
                for (sat, data) in msm
                    .satellites
                    .iter_mut()
                    .zip(m.data_segment.satellite_data.iter())
                {
                    sat.glonass_channel = data.glonass_satellite_frequency_channel_number;
                }
                msm
            }
            Message::Msg1086(m) => msm46!(
                m,
                Constellation::Glonass,
                glonass_tow_ms(self.reference(), m.glo_epoch_time_ms as i64),
                gnss_signal_fine_pseudorange_ext_ms,
                gnss_signal_fine_phaserange_ext_ms,
                gnss_phaserange_lock_time_ext_ind,
                gnss_signal_cnr_ext_dbhz
            ),
            Message::Msg1087(m) => {
                let mut msm = msm57!(
                    m,
                    Constellation::Glonass,
                    glonass_tow_ms(self.reference(), m.glo_epoch_time_ms as i64),
                    gnss_signal_fine_pseudorange_ext_ms,
                    gnss_signal_fine_phaserange_ext_ms,
                    gnss_phaserange_lock_time_ext_ind,
                    gnss_signal_cnr_ext_dbhz
                );
                for (sat, data) in msm
                    .satellites
                    .iter_mut()
                    .zip(m.data_segment.satellite_data.iter())
                {
                    sat.glonass_channel = data.glonass_satellite_frequency_channel_number;
                }
                msm
            }
            Message::Msg1104(m) => msm46!(
                m,
                Constellation::SBAS,
                m.gps_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ms,
                gnss_signal_fine_phaserange_ms,
                gnss_phaserange_lock_time_ind,
                gnss_signal_cnr_dbhz
            ),
            Message::Msg1105(m) => msm57!(
                m,
                Constellation::SBAS,
                m.gps_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ms,
                gnss_signal_fine_phaserange_ms,
                gnss_phaserange_lock_time_ind,
                gnss_signal_cnr_dbhz
            ),
            Message::Msg1106(m) => msm46!(
                m,
                Constellation::SBAS,
                m.gps_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ext_ms,
                gnss_signal_fine_phaserange_ext_ms,
                gnss_phaserange_lock_time_ext_ind,
                gnss_signal_cnr_ext_dbhz
            ),
            Message::Msg1107(m) => msm57!(
                m,
                Constellation::SBAS,
                m.gps_epoch_time_ms as i64,
                gnss_signal_fine_pseudorange_ext_ms,
                gnss_signal_fine_phaserange_ext_ms,
                gnss_phaserange_lock_time_ext_ind,
                gnss_signal_cnr_ext_dbhz
            ),
            Message::Msg1004(m) => {
                if self.redundant(Constellation::GPS) {
                    return;
                }
                self.stats.legacy += 1;
                Msm {
                    constellation: Constellation::GPS,
                    gps_tow_ms: m.gps_epoch_time_ms as i64,
                    legacy: true,
                    satellites: legacy_satellites(m.satellites.iter().map(|s| s.gps_satellite_id)),
                    signals: m
                        .satellites
                        .iter()
                        .flat_map(|sat| {
                            legacy_signals(
                                sat.gps_satellite_id,
                                if sat.gps_l1_code_ind == 0 { 'C' } else { 'P' },
                                Some(match sat.gps_l2_code_ind {
                                    0 => 'X',
                                    1 => 'P',
                                    2 => 'D',
                                    _ => 'W',
                                }),
                                sat.l1_pseudorange_m
                                    .value()
                                    .map(|pr| pr + sat.l1_pseudorange_amb_m),
                                sat.l1_phase_pseudorange_diff_m.value(),
                                sat.l1_lock_time_index,
                                sat.l1_cnr_dbhz.value(),
                                sat.l2_l1_pseudorange_diff_m.value(),
                                sat.l2_phase_l1_pseudorange_diff_m.value(),
                                sat.l2_lock_time_index,
                                sat.l2_cnr_dbhz.value(),
                            )
                        })
                        .collect(),
                }
            }
            Message::Msg1012(m) => {
                for sat in m.satellites.iter() {
                    self.glonass_channels
                        .insert(sat.glo_satellite_id, sat.glo_satellite_freq_chan_number);
                }
                if self.redundant(Constellation::Glonass) {
                    return;
                }
                self.stats.legacy += 1;
                Msm {
                    constellation: Constellation::Glonass,
                    gps_tow_ms: glonass_tow_ms(self.reference(), m.glo_epoch_time_ms as i64),
                    legacy: true,
                    satellites: legacy_satellites(m.satellites.iter().map(|s| s.glo_satellite_id)),
                    signals: m
                        .satellites
                        .iter()
                        .flat_map(|sat| {
                            legacy_signals(
                                sat.glo_satellite_id,
                                if sat.glo_l1_code_ind == 0 { 'C' } else { 'P' },
                                Some(if sat.glo_l2_code_ind == 0 { 'C' } else { 'P' }),
                                sat.l1_pseudorange_m
                                    .value()
                                    .map(|pr| pr + sat.l1_pseudorange_amb_m),
                                sat.l1_phase_pseudorange_diff_m.value(),
                                sat.l1_lock_time_index,
                                sat.l1_cnr_dbhz.value(),
                                sat.l2_l1_pseudorange_diff_m.value(),
                                sat.l2_phase_l1_pseudorange_diff_m.value(),
                                sat.l2_lock_time_index,
                                sat.l2_cnr_dbhz.value(),
                            )
                        })
                        .collect(),
                }
            }
            Message::Msg1020(m) => {
                let sv = SV::new(Constellation::Glonass, m.glo_satellite_id);
                let channel = m.glo_satellite_freq_chan_number;
                self.glonass_channels.insert(m.glo_satellite_id, channel);

                // Glonass ephemerides: PZ-90 state vector at tb (Glonass time of day),
                // RINEX conventions: clock bias = -TauN, clock drift = +GammaN
                let mut ephemeris =
                    OrbitBuilder::new(Constellation::Glonass, -m.tau_n_s, m.gamma_n as f64, 0.0)
                        .f64("satPosX", m.xn_km)
                        .f64("velX", m.xn_first_deriv_km_s)
                        .f64("accelX", m.xn_second_deriv_km_s2 as f64)
                        .f64("health", m.glo_eph_health_flag as f64)
                        .f64("satPosY", m.yn_km)
                        .f64("velY", m.yn_first_deriv_km_s)
                        .f64("accelY", m.yn_second_deriv_km_s2 as f64)
                        .f64("satPosZ", m.zn_km)
                        .f64("velZ", m.zn_first_deriv_km_s)
                        .f64("accelZ", m.zn_second_deriv_km_s2 as f64)
                        .f64("ageOp", m.en_d as f64)
                        .ephemeris;

                ephemeris
                    .orbits
                    .insert("channel".to_string(), OrbitItem::I8(channel));

                let toc = glonass_epoch(self.reference(), m.tb_min as i64 * 60_000);
                self.ephemeris(sv, toc, ephemeris, items);
                return;
            }
            Message::Msg1008(m) => {
                self.antenna(
                    &m.antenna_descriptor_str.chars().collect::<String>(),
                    &m.antenna_serial_number_str.chars().collect::<String>(),
                );
                return;
            }
            Message::Msg1033(m) => {
                self.antenna(
                    &m.antenna_descriptor_str.chars().collect::<String>(),
                    &m.antenna_serial_number_str.chars().collect::<String>(),
                );
                let receiver = [
                    &m.receiver_type_descriptor_str.chars().collect::<String>(),
                    &m.receiver_firmware_version_str.chars().collect::<String>(),
                    &m.receiver_serial_number_str.chars().collect::<String>(),
                ]
                .iter()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ");

                if !receiver.is_empty() && self.stats.receiver.as_deref() != Some(&receiver) {
                    #[cfg(feature = "log")]
                    debug!("rtcm: receiver {}", receiver);
                    self.stats.receiver = Some(receiver);
                }
                return;
            }
            Message::Msg1230(m) => {
                for bias in m.glo_code_phase_biases.iter() {
                    let signal = format!("{}{}", bias.signal_id.band(), bias.signal_id.attribute());
                    self.stats
                        .glonass_code_phase_biases
                        .insert(signal, bias.bias_m as f64);
                }
                return;
            }
            other => {
                let number = other.number().unwrap_or_default();
                let count = self.stats.ignored.entry(number).or_default();
                *count += 1;

                #[cfg(feature = "log")]
                if *count == 1 {
                    debug!("rtcm: message {} is not interpreted", number);
                }
                return;
            }
        };

        self.observations(msm, items);
    }

    /// Legacy observations of a constellation streamed as MSM too
    fn redundant(&mut self, constellation: Constellation) -> bool {
        if !self.msm_constellations.contains(&constellation) {
            return false;
        }

        self.stats.redundant += 1;

        #[cfg(feature = "log")]
        if self.stats.redundant == 1 {
            debug!(
                "rtcm: {} legacy observations skipped (MSM streamed too)",
                constellation
            );
        }

        true
    }

    /// Antenna descriptor (1008, 1033)
    fn antenna(&mut self, descriptor: &str, serial: &str) {
        let antenna = [descriptor.trim(), serial.trim()]
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");

        if !antenna.is_empty() && self.stats.antenna.as_deref() != Some(&antenna) {
            #[cfg(feature = "log")]
            debug!("rtcm: antenna {}", antenna);
            self.stats.antenna = Some(antenna);
        }
    }

    /// Context data (ephemerides, station coordinates): held until the first observation
    /// of a real-time stream (see the time tags), released otherwise
    fn context(&mut self, payload: QcPayload, items: &mut Vec<QcInputItem>) {
        if self.reference.is_some() {
            items.push(QcInputItem {
                epoch: self.reference(),
                payload,
            });
            return;
        }

        if self.pending_since.is_none() {
            self.pending_since = Some(std::time::Instant::now());
        }

        self.pending.push(QcInputItem {
            epoch: self.reference(),
            payload,
        });

        self.release_pending(None, items);
    }

    /// Releases the pending context data: tagged with the first observation epoch,
    /// or with the system time after [PENDING_CONTEXT] without observation
    fn release_pending(&mut self, epoch: Option<Epoch>, items: &mut Vec<QcInputItem>) {
        if self.pending.is_empty() {
            return;
        }

        let epoch = match epoch {
            Some(epoch) => epoch,
            None => {
                if self
                    .pending_since
                    .is_none_or(|t| t.elapsed() < PENDING_CONTEXT)
                {
                    return;
                }
                self.reference()
            }
        };

        for mut item in self.pending.drain(..) {
            item.epoch = epoch;
            items.push(item);
        }

        self.pending_since = None;
    }

    /// Station coordinates (antenna reference point)
    fn station(&mut self, ecef_m: (f64, f64, f64), items: &mut Vec<QcInputItem>) {
        self.stats.stations += 1;
        self.context(QcPayload::ReferencePosition(ecef_m), items);
    }

    /// Radio ephemeris, time tagged with its reception time
    fn ephemeris(
        &mut self,
        sv: SV,
        toc: Epoch,
        ephemeris: Ephemeris,
        items: &mut Vec<QcInputItem>,
    ) {
        self.stats.ephemerides += 1;
        self.context(
            QcPayload::Ephemeris(QcEphemeris {
                sv,
                toc,
                ephemeris: Box::new(ephemeris),
            }),
            items,
        );
    }

    /// MSM observations
    fn observations(&mut self, msm: Msm, items: &mut Vec<QcInputItem>) {
        let epoch = resolve_tow(self.reference(), msm.gps_tow_ms);

        self.release_pending(Some(epoch), items);

        self.reference = Some(epoch);

        if self.delivered.0 != Some(epoch) {
            self.delivered.0 = Some(epoch);
            self.delivered.1.clear();
        }

        if !msm.legacy {
            self.stats.msm += 1;
            self.msm_constellations.insert(msm.constellation);
        }

        for signal in msm.signals.iter() {
            let satellite = match msm.satellites.iter().find(|s| s.id == signal.satellite) {
                Some(satellite) => satellite,
                None => continue,
            };

            let rough_ms = match satellite.rough_ms {
                Some(rough_ms) => rough_ms,
                None => continue,
            };

            let sv = match msm.constellation {
                // MSM satellite 1 is the SBAS PRN 120 (RINEX: S20)
                Constellation::SBAS => {
                    match SV::from_str(&format!("S{:02}", signal.satellite as u16 + 19)) {
                        Ok(sv) => sv,
                        Err(_) => continue,
                    }
                }
                constellation => SV::new(constellation, signal.satellite),
            };

            let glonass_channel = satellite.glonass_channel.or_else(|| {
                (msm.constellation == Constellation::Glonass)
                    .then(|| self.glonass_channels.get(&signal.satellite).copied())
                    .flatten()
            });

            if let (Some(channel), Constellation::Glonass) = (glonass_channel, msm.constellation) {
                self.glonass_channels.insert(signal.satellite, channel);
            }

            let code = format!("{}{}", signal.band, signal.attribute);

            let snr = signal.cnr_dbhz.map(SNR::from);

            // loss of lock: the lock time indicator decreased
            let lock_loss = match self
                .locks
                .insert((sv, signal.band, signal.attribute), signal.lock)
            {
                Some(previous) => signal.lock < previous,
                None => false,
            };

            let mut lli = LliFlags::OK_OR_UNKNOWN;
            if lock_loss {
                lli |= LliFlags::LOCK_LOSS;
            }
            if signal.half_cycle {
                lli |= LliFlags::HALF_CYCLE_SLIP;
            }

            let delivered = &mut self.delivered.1;

            let mut push = |observable: Observable, value: f64, lli: Option<LliFlags>| {
                if !delivered.insert((sv, observable.to_string())) {
                    return; // already delivered (legacy and MSM)
                }
                self.stats.signals += 1;
                items.push(QcInputItem::signal(
                    epoch,
                    EpochFlag::Ok,
                    SignalObservation {
                        sv,
                        value,
                        observable,
                        lli,
                        snr,
                    },
                ));
            };

            if let Some(fine_ms) = signal.pseudo_range_ms {
                push(
                    Observable::PseudoRange(format!("C{}", code)),
                    (rough_ms + fine_ms) * LIGHT_MS,
                    None,
                );
            }

            let wavelength_m = wavelength_m(
                msm.constellation,
                signal.band,
                signal.attribute,
                glonass_channel,
            );

            if let (Some(fine_ms), Some(wavelength_m)) = (signal.phase_range_ms, wavelength_m) {
                push(
                    Observable::PhaseRange(format!("L{}", code)),
                    (rough_ms + fine_ms) * LIGHT_MS / wavelength_m,
                    Some(lli),
                );
            }

            if let (Some(rough_rate), Some(fine_rate), Some(wavelength_m)) =
                (satellite.rough_rate_m_s, signal.rate_m_s, wavelength_m)
            {
                push(
                    Observable::Doppler(format!("D{}", code)),
                    -(rough_rate + fine_rate) / wavelength_m,
                    None,
                );
            }

            if let Some(cnr_dbhz) = signal.cnr_dbhz {
                push(Observable::SSI(format!("S{}", code)), cnr_dbhz, None);
            }
        }
    }
}

/// [QcSource] over recorded RTCM 3 data (file, capture of a stream..).
/// For real-time streams, see the NTRIP client (`ntrip` feature).
pub struct QcRtcmSource {
    descriptor: QcSourceDescriptor,
    decoder: QcRtcmDecoder,
    bytes: Vec<u8>,
    offset: usize,
    items: std::collections::VecDeque<QcInputItem>,
}

impl QcRtcmSource {
    /// Builds a new [QcRtcmSource] from recorded RTCM `bytes`.
    /// The `descriptor` tells the engine what to expect (signals, ephemerides..),
    /// the `decoder` should know the approximate date of the record
    /// ([QcRtcmDecoder::with_reference_epoch]).
    pub fn new(descriptor: QcSourceDescriptor, decoder: QcRtcmDecoder, bytes: Vec<u8>) -> Self {
        Self {
            descriptor,
            decoder,
            bytes,
            offset: 0,
            items: Default::default(),
        }
    }

    /// Statistics of the decoded stream
    pub fn stats(&self) -> &QcRtcmStats {
        self.decoder.stats()
    }
}

impl QcSource for QcRtcmSource {
    fn descriptor(&self) -> &QcSourceDescriptor {
        &self.descriptor
    }

    fn poll(&mut self) -> Result<QcPoll<QcInputItem>, QcSourceError> {
        let mut items = Vec::new();

        while self.items.is_empty() && self.offset < self.bytes.len() {
            // one frame at a time: memory remains bounded
            let end = (self.offset + 1029).min(self.bytes.len());
            let consumed = self
                .decoder
                .decode_bytes(&self.bytes[self.offset..end], &mut items);

            self.offset += if consumed == 0 {
                end - self.offset // truncated frame at the end of the record
            } else {
                consumed
            };

            self.items.extend(items.drain(..));
        }

        Ok(match self.items.pop_front() {
            Some(item) => QcPoll::Ready(item),
            None => QcPoll::EndOfStream,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    fn epoch(s: &str) -> Epoch {
        Epoch::from_str(s).unwrap()
    }

    fn tow_ms(s: &str) -> i64 {
        (epoch(s).to_time_of_week().1 / 1_000_000) as i64
    }

    #[test]
    fn week_rollover() {
        for (reference, t) in [
            // same week
            ("2020-06-25T00:00:00 GPST", "2020-06-25T00:00:01 GPST"),
            // end of week, reference just after the rollover
            ("2020-06-28T00:00:05 GPST", "2020-06-27T23:59:59 GPST"),
            // start of week, reference just before the rollover
            ("2020-06-27T23:59:50 GPST", "2020-06-28T00:00:01 GPST"),
            // three days apart
            ("2020-06-25T00:00:00 GPST", "2020-06-28T00:00:00 GPST"),
            ("2020-06-28T00:00:00 GPST", "2020-06-25T00:00:00 GPST"),
        ] {
            assert_eq!(
                resolve_tow(epoch(reference), tow_ms(t)),
                epoch(t),
                "reference {}",
                reference
            );
        }
    }

    #[test]
    fn truncated_weeks() {
        // GPS week 2111 (2020-06-25) transmitted modulo 1024
        assert_eq!(resolve_week(2111, 2111 % 1024, 1024), 2111);
        // reference just after a rollover of the truncated counter
        assert_eq!(resolve_week(2048, 1023, 1024), 2047);
        assert_eq!(resolve_week(2047, 0, 1024), 2048);
        // Galileo: GST week + 1024, modulo 4096
        assert_eq!(resolve_week(2111, 1087 + GST_GPS_WEEK_OFFSET, 4096), 2111);
    }

    #[test]
    fn accuracy_indices() {
        assert_eq!(ura_meters(0), 2.4);
        assert_eq!(ura_meters(15), -1.0);
        assert_eq!(sisa_meters(49), 0.49);
        assert_eq!(sisa_meters(255), -1.0);
    }
}
