//! Total Electron Content (TEC) estimated from the signal observations:
//! dual frequency code and phase, leveled phase, single layer model.
use std::collections::{BTreeMap, HashMap};

use gnss_rs::prelude::{Constellation, SV};
use hifitime::{Duration, Epoch};
use ionex::prelude::{Point, IPP};

use rinex::prelude::{obs::SignalObservation, Observable};

use crate::{
    engine::{QcEpochContext, QcProcessor},
    geodesy::ecef_to_geodetic,
    io::{
        input::indexing::QcIndexing,
        output::{QcOutput, QcOutputQueue, QcTecArc, QcTecEstimate},
    },
    orbit::{satellite_state, QcOrbitPreference},
    processors::{combination::bands, elevation_azimuth_deg},
    sync::QcTecInterpolation,
};

/// Ionospheric delay constant: I = 40.3 TEC / f² (m, el/m², Hz)
const K_IONO: f64 = 40.3;

/// One TECu (electrons per m²)
const TECU: f64 = 1.0E16;

/// Code preference for the TEC: precise codes first, consistent with the
/// broadcast group delays (P1 - P2 for GPS)
const TEC_ATTRIBUTES: &str = "WPYCXQIDLSBMZAN";

/// Slant TEC (TECu) per meter of ionospheric delay difference `Ij − I1`
pub fn tecu_per_meter(f1_hz: f64, fj_hz: f64) -> f64 {
    let (f1, fj) = (f1_hz.powi(2), fj_hz.powi(2));
    f1 * fj / (K_IONO * (f1 - fj)) / TECU
}

/// Preferred second frequency (RINEX band) of each constellation, with the
/// first frequency being the lowest band
fn preferred_bands(constellation: Constellation) -> &'static [char] {
    match constellation {
        Constellation::GPS | Constellation::QZSS => &['2', '5'],
        Constellation::Galileo => &['5', '7', '8', '6'],
        Constellation::BeiDou => &['6', '7', '5', '8'],
        _ => &['5', '2'],
    }
}

/// Configuration of the TEC estimation
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct QcObsTecConfig {
    /// Orbital products preference. Default: precise products whenever available.
    pub orbits: QcOrbitPreference,

    /// Elevation mask (degrees). Default: 15° (mapping function errors grow at low elevation).
    pub elevation_mask_deg: f64,

    /// Altitude of the ionospheric shell (single layer model, km). Default: 450 km (IONEX).
    pub shell_altitude_km: f64,

    /// Radius of the (spherical) Earth (km). Default: 6371 km (IONEX).
    pub earth_radius_km: f64,

    /// Receivers to process. Default (empty): all of them.
    pub receivers: Vec<QcIndexing>,

    /// Constellations to process. Default (empty): all of them (Glonass excluded: FDMA).
    pub constellations: Vec<Constellation>,

    /// Receiver position (ECEF m). Default: the reference position of each signal source.
    /// Receivers without position are not processed.
    pub receiver_position_ecef_m: Option<(f64, f64, f64)>,

    /// A data gap longer than this ends a phase arc. Default: 10 minutes.
    pub max_gap: Duration,

    /// A jump of the Geometry Free phase larger than this (m) between two epochs
    /// ends a phase arc (cycle slip). Default: 0.1 m.
    pub slip_threshold_m: f64,
}

impl Default for QcObsTecConfig {
    fn default() -> Self {
        Self {
            orbits: QcOrbitPreference::Auto,
            elevation_mask_deg: 15.0,
            shell_altitude_km: 450.0,
            earth_radius_km: 6371.0,
            receivers: Vec::new(),
            constellations: Vec::new(),
            receiver_position_ecef_m: None,
            max_gap: Duration::from_seconds(600.0),
            slip_threshold_m: 0.1,
        }
    }
}

impl QcObsTecConfig {
    /// Only this receiver
    pub fn with_receiver(mut self, receiver: QcIndexing) -> Self {
        self.receivers.push(receiver);
        self
    }

    /// Only these constellations
    pub fn with_constellations(mut self, constellations: &[Constellation]) -> Self {
        self.constellations = constellations.to_vec();
        self
    }
}

/// Phase arc of one line of sight (leveling)
#[derive(Debug, Clone)]
struct Arc {
    id: u32,
    start: Epoch,
    last: Epoch,
    last_gf_phase_m: f64,
    samples: usize,
    /// running mean of (code GF − phase GF), m
    offset_m: f64,
    /// running variance accumulator of (code GF − phase GF), m²
    m2: f64,
    /// TECu per meter of ionospheric delay difference
    tecu_per_m: f64,
    /// observables of the line of sight
    phase_1: Observable,
    phase_j: Observable,
}

/// (receiver, satellite, code 1, code j)
type ArcKey = (QcIndexing, SV, Observable, Observable);

/// Estimates the Total Electron Content from the signal observations of every receiver
/// (signal source), at the Ionospheric Pierce Point (IPP) of each line of sight.
/// It does not produce TEC maps: each estimate is a point (epoch, pierce point, STEC, VTEC),
/// to be collected (gridded, interpolated, formatted as IONEX..) by the application.
///
/// 1. For each satellite, two frequencies are selected: the lowest band (L1, E1, B1I..) and
///    a second band (L2 then L5 for GPS, E5a then E5b, E5, E6 for Galileo, B3I for BeiDou..),
///    with a code and a phase on each (precise codes first: `W`, `P`, `Y`, then `C`..).
/// 2. Slant TEC, with `k = f1² fj² / (40.3 (f1² − fj²))` (el/m² per meter, TECu: 10^16 el/m²):
///    - from the codes: `STEC(code) = k (Cj − C1)`: absolute, but noisy (code noise, multipath)
///    - from the phases: `STEC(phase) = k (L1 − Lj)` (L in meters): precise, but ambiguous
///      (unknown integer number of cycles on each frequency)
/// 3. Leveling: over each continuous phase arc, the phase is shifted onto the code:
///    `STEC = STEC(phase) + < STEC(code) − STEC(phase) >arc`. The mean is a running mean (causal,
///    real-time compatible): the first samples of an arc are less accurate. When the arc ends
///    (loss of lock, data gap, cycle slip, end of stream), its final mean is emitted
///    ([QcTecArc]): `STEC(phase) + leveling` gives the exact leveling of every sample of the arc.
/// 4. Differential code biases: the complete formula subtracts `k c (DCB(sat) + DCB(rx))`.
///    They are not corrected yet (null term, see `DCB_M`): the TEC is *uncalibrated*, biased by
///    the code biases of the satellite and of the receiver (2.85 TECu per ns for GPS L1/L2).
/// 5. Single layer model: the TEC is located at the pierce point of the line of sight
///    with a thin shell (450 km, as IONEX), and mapped to the vertical with the obliquity factor:
///    `VTEC = STEC cos(z')`, `sin(z') = R / (R + H) sin(z)`, z being the zenith angle
///    at the receiver.
///
/// Process the observations of a whole network of receivers to cover a region: each receiver
/// contributes its pierce points. When TEC maps (IONEX) are part of the inputs, their vertical
/// TEC at the same pierce point is reported next to each estimate.
#[derive(Debug, Default, Clone)]
pub struct QcObsTecProcessor {
    cfg: QcObsTecConfig,
    arcs: HashMap<ArcKey, Arc>,
    next_arc: HashMap<ArcKey, u32>,
    last_epoch: Option<Epoch>,
}

/// Differential code biases of the satellite and of the receiver, c (DCB(sat) + DCB(rx)), in
/// meters. Not corrected yet: they will be when GNSS-Qc accepts DCB products (Bias SINEX,
/// group delays of the radio ephemerides, receiver calibrations). Until then, the TEC estimates
/// are uncalibrated.
const DCB_M: f64 = 0.0;

/// Leveling of a line of sight, at one epoch
struct Leveling {
    arc: u32,
    samples: usize,
    offset_m: f64,
}

impl QcObsTecProcessor {
    /// Builds a new [QcObsTecProcessor]
    pub fn new(cfg: QcObsTecConfig) -> Self {
        Self {
            cfg,
            ..Default::default()
        }
    }

    /// Emits the statistics of an arc that ended
    fn close<O: From<QcOutput>>(
        epoch: Epoch,
        key: &ArcKey,
        arc: &Arc,
        output: &mut QcOutputQueue<O>,
    ) {
        if arc.samples == 0 {
            return;
        }

        output.push(
            epoch,
            QcOutput::TecArc(Box::new(QcTecArc {
                receiver: key.0.clone(),
                sv: key.1,
                code_1: key.2.clone(),
                code_j: key.3.clone(),
                phase_1: arc.phase_1.clone(),
                phase_j: arc.phase_j.clone(),
                arc: arc.id,
                start: arc.start,
                end: arc.last,
                samples: arc.samples,
                leveling_tecu: arc.offset_m * arc.tecu_per_m,
                code_noise_tecu: (arc.m2 / arc.samples as f64).sqrt() * arc.tecu_per_m,
            })),
        );
    }

    /// Updates the arc of this line of sight
    #[allow(clippy::too_many_arguments)]
    fn level<O: From<QcOutput>>(
        &mut self,
        epoch: Epoch,
        key: ArcKey,
        observables: (&Observable, &Observable),
        tecu_per_m: f64,
        code_gf_m: f64,
        phase_gf_m: f64,
        lock_loss: bool,
        output: &mut QcOutputQueue<O>,
    ) -> Leveling {
        let restart = match self.arcs.get(&key) {
            Some(arc) => {
                lock_loss
                    || epoch - arc.last > self.cfg.max_gap
                    || (phase_gf_m - arc.last_gf_phase_m).abs() > self.cfg.slip_threshold_m
            }
            None => true,
        };

        if restart {
            if let Some(arc) = self.arcs.remove(&key) {
                Self::close(epoch, &key, &arc, output);
            }

            let id = self.next_arc.entry(key.clone()).or_default();
            self.arcs.insert(
                key.clone(),
                Arc {
                    id: *id,
                    start: epoch,
                    last: epoch,
                    last_gf_phase_m: phase_gf_m,
                    samples: 0,
                    offset_m: 0.0,
                    m2: 0.0,
                    tecu_per_m,
                    phase_1: observables.0.clone(),
                    phase_j: observables.1.clone(),
                },
            );
            *id += 1;
        }

        let arc = self.arcs.get_mut(&key).unwrap();
        arc.last = epoch;
        arc.last_gf_phase_m = phase_gf_m;
        arc.samples += 1;

        // Welford: running mean & variance of code − phase
        let difference = code_gf_m - phase_gf_m;
        let delta = difference - arc.offset_m;
        arc.offset_m += delta / arc.samples as f64;
        arc.m2 += delta * (difference - arc.offset_m);

        Leveling {
            arc: arc.id,
            samples: arc.samples,
            offset_m: arc.offset_m,
        }
    }

    /// Closes the arcs that were not updated for too long (satellite set, receiver lost)
    fn close_stale_arcs<O: From<QcOutput>>(&mut self, epoch: Epoch, output: &mut QcOutputQueue<O>) {
        let max_gap = self.cfg.max_gap;

        let mut stale = self
            .arcs
            .iter()
            .filter(|(_, arc)| epoch - arc.last > max_gap)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();

        stale.sort_by_key(sort_key);

        for key in stale {
            if let Some(arc) = self.arcs.remove(&key) {
                Self::close(epoch, &key, &arc, output);
            }
        }
    }
}

/// Deterministic order of the arcs
fn sort_key(key: &ArcKey) -> (String, SV, Observable, Observable) {
    (key.0.to_string(), key.1, key.2.clone(), key.3.clone())
}

impl<O: From<QcOutput>> QcProcessor<O> for QcObsTecProcessor {
    fn name(&self) -> &str {
        "obs-tec"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>) {
        self.last_epoch = Some(ctx.epoch);

        for rx in ctx.receivers {
            if !self.cfg.receivers.is_empty() && !self.cfg.receivers.contains(&rx.receiver) {
                continue;
            }

            let Some(position) = self
                .cfg
                .receiver_position_ecef_m
                .or(rx.reference_position_ecef_m)
            else {
                continue;
            };

            let (lat_rad, long_rad, _) = ecef_to_geodetic(position);
            let receiver_point = Point::new(long_rad.to_degrees(), lat_rad.to_degrees());

            let mut satellites = BTreeMap::<SV, Vec<&SignalObservation>>::new();
            for signal in rx.signals.iter() {
                let constellation = signal.sv.constellation;
                if constellation == Constellation::Glonass {
                    continue;
                }
                if !self.cfg.constellations.is_empty()
                    && !self.cfg.constellations.contains(&constellation)
                {
                    continue;
                }
                satellites.entry(signal.sv).or_default().push(signal);
            }

            for (sv, signals) in satellites.iter() {
                let bands = bands(*sv, signals);

                // first frequency: lowest band with code and phase
                let Some((band_1, b1)) = bands.iter().find(|(_, b)| {
                    b.code_by(TEC_ATTRIBUTES).is_some() && b.phase_by(TEC_ATTRIBUTES).is_some()
                }) else {
                    continue;
                };

                // second frequency: by preference, then any other band
                let other = preferred_bands(sv.constellation)
                    .iter()
                    .filter(|band| *band != band_1)
                    .filter_map(|band| bands.get_key_value(band))
                    .chain(bands.iter().filter(|(band, _)| *band != band_1))
                    .find(|(_, b)| {
                        b.code_by(TEC_ATTRIBUTES).is_some() && b.phase_by(TEC_ATTRIBUTES).is_some()
                    });

                let Some((_, bj)) = other else {
                    continue;
                };

                let (c1_obs, c1) = b1.code_by(TEC_ATTRIBUTES).unwrap();
                let (cj_obs, cj) = bj.code_by(TEC_ATTRIBUTES).unwrap();
                let (l1_obs, l1, l1_lock_loss) = b1.phase_by(TEC_ATTRIBUTES).unwrap();
                let (lj_obs, lj, lj_lock_loss) = bj.phase_by(TEC_ATTRIBUTES).unwrap();

                // geometry: elevation mask, pierce point
                let Some(state) = satellite_state(ctx, *sv, ctx.epoch, self.cfg.orbits) else {
                    continue;
                };

                let (elevation_deg, azimuth_deg) =
                    elevation_azimuth_deg(position, state.position_m);

                if elevation_deg < self.cfg.elevation_mask_deg {
                    continue;
                }

                let Ok(ipp) = IPP::new(
                    receiver_point,
                    elevation_deg,
                    azimuth_deg,
                    self.cfg.earth_radius_km,
                    self.cfg.shell_altitude_km,
                ) else {
                    continue;
                };

                // ionospheric delay difference Ij − I1: codes (absolute), phases (ambiguous)
                let code_gf_m = cj - c1 - DCB_M;
                let phase_gf_m = l1 - lj;

                let tecu_per_m = tecu_per_meter(b1.frequency_hz, bj.frequency_hz);

                let key = (rx.receiver.clone(), *sv, c1_obs.clone(), cj_obs.clone());

                let leveling = self.level(
                    ctx.epoch,
                    key,
                    (l1_obs, lj_obs),
                    tecu_per_m,
                    code_gf_m,
                    phase_gf_m,
                    *l1_lock_loss || *lj_lock_loss,
                    output,
                );

                let phase_slant_tecu = phase_gf_m * tecu_per_m;
                let leveling_tecu = leveling.offset_m * tecu_per_m;
                let slant_tecu = phase_slant_tecu + leveling_tecu;

                let obliquity_factor = ipp.obliquity_factor();
                let (latitude_deg, longitude_deg) = (ipp.coordinates.y(), ipp.coordinates.x());

                output.push(
                    ctx.epoch,
                    QcOutput::TecEstimate(Box::new(QcTecEstimate {
                        receiver: rx.receiver.clone(),
                        sv: *sv,
                        code_1: c1_obs.clone(),
                        code_j: cj_obs.clone(),
                        phase_1: l1_obs.clone(),
                        phase_j: lj_obs.clone(),
                        arc: leveling.arc,
                        arc_samples: leveling.samples,
                        elevation_deg,
                        azimuth_deg,
                        latitude_deg,
                        longitude_deg,
                        altitude_km: ipp.altitude_km,
                        obliquity_factor,
                        code_slant_tecu: code_gf_m * tecu_per_m,
                        phase_slant_tecu,
                        leveling_tecu,
                        slant_tecu,
                        vertical_tecu: slant_tecu / obliquity_factor,
                        map_vertical_tecu: ctx
                            .tec_maps
                            .tec(
                                ctx.epoch,
                                latitude_deg,
                                longitude_deg,
                                QcTecInterpolation::RotatedMaps,
                            )
                            .map(|(_, value)| value.tecu),
                    })),
                );
            }
        }

        self.close_stale_arcs(ctx.epoch, output);
    }

    fn finalize(&mut self, output: &mut QcOutputQueue<O>) {
        let mut keys = self.arcs.keys().cloned().collect::<Vec<_>>();
        keys.sort_by_key(sort_key);

        for key in keys {
            if let Some(arc) = self.arcs.remove(&key) {
                Self::close(self.last_epoch.unwrap_or(arc.last), &key, &arc, output);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tec_conversion() {
        // GPS L1/L2: 9.52 TECu per meter, 2.85 TECu per ns
        let k = tecu_per_meter(1575.42E6, 1227.60E6);
        assert!((k - 9.52).abs() < 0.01, "{}", k);
        assert!((k * 299_792_458.0 * 1.0E-9 - 2.854).abs() < 0.01);

        // 10 TECu: I1 = 1.62 m, I2 = 2.67 m
        let i = |tec: f64, f: f64| K_IONO * tec * TECU / f.powi(2);
        let (i1, i2) = (i(10.0, 1575.42E6), i(10.0, 1227.60E6));
        assert!((i1 - 1.624).abs() < 1.0E-3, "{}", i1);
        assert!(((i2 - i1) * k - 10.0).abs() < 1.0E-9);

        // Galileo E1/E5a
        let k = tecu_per_meter(1575.42E6, 1176.45E6);
        assert!((k - 7.76).abs() < 0.01, "{}", k);
    }

    #[test]
    fn bands_preference() {
        assert_eq!(preferred_bands(Constellation::GPS)[0], '2');
        assert_eq!(preferred_bands(Constellation::Galileo)[0], '5');
        assert_eq!(preferred_bands(Constellation::BeiDou)[0], '6');
    }
}
