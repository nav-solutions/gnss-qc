//! RTCM 3 inputs.
//!
//! [QcRtcmDecoder] converts RTCM 3 messages (decoded by [rtcm_rs]) into the data points
//! of the MISO engine ([QcInputItem]). It does not perform any I/O, so it serves RTCM files,
//! serial ports and network streams alike (see the NTRIP client, `ntrip` feature):
//!
//! | Messages | Constellation | Data points |
//! |----------|---------------|-------------|
//! | 1074 - 1077 (MSM4 - MSM7) | GPS | Signals: pseudo range, phase, doppler (MSM5/7), C/N0 |
//! | 1094 - 1097 | Galileo | idem |
//! | 1114 - 1117 | QZSS | idem |
//! | 1124 - 1127 | BeiDou | idem |
//! | 1019 | GPS | Radio ephemeris |
//! | 1045 (F/NAV), 1046 (I/NAV) | Galileo | Radio ephemeris |
//! | 1042 | BeiDou | Radio ephemeris |
//! | 1044 | QZSS | Radio ephemeris |
//! | 1005, 1006 | - | Reference position of the station (antenna reference point) |
//!
//! The other messages (Glonass, SBAS, legacy observations, SSR..) are counted in [QcRtcmStats]
//! but not interpreted yet.
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
//!
//! ## Signals
//! MSM signals are expressed as RINEX observations, in RINEX units:
//! `C1C` (pseudo range, m), `L1C` (phase, cycles), `D1C` (doppler, Hz), `S1C` (C/N0, dB-Hz).
//! MSM phase ranges are aligned with the pseudo ranges by the receiver (RTCM 3.3 standard),
//! the lock time indicators are converted to loss of lock flags.
use std::collections::{BTreeMap, HashMap};

use gnss_rs::prelude::{Constellation, SV};
use hifitime::{Epoch, TimeScale};

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

    /// Station coordinates (antenna reference point)
    fn station(&mut self, ecef_m: (f64, f64, f64), items: &mut Vec<QcInputItem>) {
        self.stats.stations += 1;
        items.push(QcInputItem::reference_position(self.reference(), ecef_m));
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
        items.push(QcInputItem {
            epoch: self.reference(),
            payload: QcPayload::Ephemeris(QcEphemeris {
                sv,
                toc,
                ephemeris: Box::new(ephemeris),
            }),
        });
    }

    /// MSM observations
    fn observations(&mut self, msm: Msm, items: &mut Vec<QcInputItem>) {
        let epoch = resolve_tow(self.reference(), msm.gps_tow_ms);

        self.reference = Some(epoch);
        self.stats.msm += 1;

        for signal in msm.signals.iter() {
            let satellite = match msm.satellites.iter().find(|s| s.id == signal.satellite) {
                Some(satellite) => satellite,
                None => continue,
            };

            let rough_ms = match satellite.rough_ms {
                Some(rough_ms) => rough_ms,
                None => continue,
            };

            let sv = SV::new(msm.constellation, signal.satellite);
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

            let mut push = |observable: Observable, value: f64, lli: Option<LliFlags>| {
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

            let wavelength_m = Observable::PhaseRange(format!("L{}", code))
                .to_carrier(msm.constellation)
                .ok()
                .map(|carrier| carrier.wavelength());

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
