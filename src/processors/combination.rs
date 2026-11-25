//! Signal combinations: Geometry Free (GF), Ionosphere Free (IF) and code multipath (MP),
//! for each receiver, satellite and pair of frequencies.
use std::collections::{BTreeMap, HashMap};

use gnss_rs::prelude::{Constellation, SV};
use hifitime::{Duration, Epoch};

use rinex::prelude::{
    obs::{LliFlags, SignalObservation},
    Observable,
};

use crate::{
    engine::{QcEpochContext, QcProcessor},
    io::{
        input::indexing::QcIndexing,
        output::{QcCombination, QcCombinationKind, QcMultipathArc, QcOutput, QcOutputQueue},
    },
};

/// Preference among the signals of one frequency (RINEX attributes):
/// civilian first, then precise codes, then the others.
const ATTRIBUTES: &str = "CWPYXQILSBDMZAN";

fn attribute_rank(attribute: char) -> usize {
    ATTRIBUTES.find(attribute).unwrap_or(ATTRIBUTES.len())
}

/// Signal combinations configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct QcCombinationConfig {
    /// Geometry Free combinations. Default: true.
    pub geometry_free: bool,

    /// Ionosphere Free combinations. Default: true.
    pub ionosphere_free: bool,

    /// Code multipath. Default: true.
    pub multipath: bool,

    /// Combine pseudo ranges (GF, IF). Default: true.
    pub code: bool,

    /// Combine phase ranges (GF, IF). Default: true.
    pub phase: bool,

    /// Receivers to analyze. Default (empty): all of them.
    pub receivers: Vec<QcIndexing>,

    /// Constellations to analyze. Default (empty): all of them.
    /// Glonass (FDMA: frequency of each satellite) is not supported.
    pub constellations: Vec<Constellation>,

    /// Multipath: a data gap longer than this ends the phase arc. Default: 10 minutes.
    pub max_gap: Duration,

    /// Multipath: a jump larger than this (m, with respect to the mean of the arc)
    /// ends the phase arc (undetected cycle slip). Default: 5 m.
    pub jump_threshold_m: f64,
}

impl Default for QcCombinationConfig {
    fn default() -> Self {
        Self {
            geometry_free: true,
            ionosphere_free: true,
            multipath: true,
            code: true,
            phase: true,
            receivers: Vec::new(),
            constellations: Vec::new(),
            max_gap: Duration::from_seconds(600.0),
            jump_threshold_m: 5.0,
        }
    }
}

impl QcCombinationConfig {
    /// Only these combinations
    pub fn with_combinations(mut self, kinds: &[QcCombinationKind]) -> Self {
        self.geometry_free = kinds.contains(&QcCombinationKind::GeometryFree);
        self.ionosphere_free = kinds.contains(&QcCombinationKind::IonosphereFree);
        self.multipath = kinds.contains(&QcCombinationKind::Multipath);
        self
    }

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

    fn selected_receiver(&self, receiver: &QcIndexing) -> bool {
        self.receivers.is_empty() || self.receivers.contains(receiver)
    }

    fn selected_constellation(&self, constellation: Constellation) -> bool {
        if constellation == Constellation::Glonass {
            return false;
        }
        self.constellations.is_empty() || self.constellations.contains(&constellation)
    }
}

/// Observations of one frequency
#[derive(Debug, Default)]
struct Band {
    frequency_hz: f64,
    wavelength_m: f64,
    /// Codes (m), per attribute
    codes: BTreeMap<char, (Observable, f64)>,
    /// Phases (m), per attribute, loss of lock
    phases: BTreeMap<char, (Observable, f64, bool)>,
}

impl Band {
    fn code(&self) -> Option<&(Observable, f64)> {
        self.codes
            .iter()
            .min_by_key(|(attribute, _)| attribute_rank(**attribute))
            .map(|(_, code)| code)
    }

    fn phase(&self) -> Option<&(Observable, f64, bool)> {
        self.phases
            .iter()
            .min_by_key(|(attribute, _)| attribute_rank(**attribute))
            .map(|(_, phase)| phase)
    }
}

/// Signals of one satellite, sorted per frequency band (RINEX band number)
fn bands(sv: SV, signals: &[&SignalObservation]) -> BTreeMap<char, Band> {
    let mut bands = BTreeMap::<char, Band>::new();

    for signal in signals.iter() {
        let code = signal.observable.to_string();
        let mut chars = code.chars();

        let (Some(_), Some(band), Some(attribute)) = (chars.next(), chars.next(), chars.next())
        else {
            continue;
        };

        let carrier = match signal.observable.to_carrier(sv.constellation) {
            Ok(carrier) => carrier,
            Err(_) => continue,
        };

        let entry = bands.entry(band).or_insert_with(|| Band {
            frequency_hz: carrier.frequency(),
            wavelength_m: carrier.wavelength(),
            ..Default::default()
        });

        match &signal.observable {
            Observable::PseudoRange(_) => {
                entry
                    .codes
                    .insert(attribute, (signal.observable.clone(), signal.value));
            }
            Observable::PhaseRange(_) => {
                let lock_loss = signal
                    .lli
                    .map(|lli| lli.intersects(LliFlags::LOCK_LOSS))
                    .unwrap_or(false);
                entry.phases.insert(
                    attribute,
                    (
                        signal.observable.clone(),
                        signal.value * entry.wavelength_m,
                        lock_loss,
                    ),
                );
            }
            _ => {}
        }
    }

    bands
}

/// Ionosphere Free combination (m)
fn ionosphere_free(f1_hz: f64, x1: f64, fj_hz: f64, xj: f64) -> f64 {
    let (f1, fj) = (f1_hz.powi(2), fj_hz.powi(2));
    (f1 * x1 - fj * xj) / (f1 - fj)
}

/// Code multipath of code `ci` (m), with the phases `li` (same frequency) and `lj` (m)
fn multipath(fi_hz: f64, ci: f64, li: f64, fj_hz: f64, lj: f64) -> f64 {
    let alpha = (fi_hz / fj_hz).powi(2);
    ci - li - 2.0 * (li - lj) / (alpha - 1.0)
}

/// Continuous phase arc of one multipath combination
#[derive(Debug, Clone)]
struct Arc {
    id: u32,
    start: Epoch,
    last: Epoch,
    samples: usize,
    mean: f64,
    m2: f64,
}

impl Arc {
    fn new(id: u32, epoch: Epoch) -> Self {
        Self {
            id,
            start: epoch,
            last: epoch,
            samples: 0,
            mean: 0.0,
            m2: 0.0,
        }
    }

    /// Welford's running mean & variance
    fn push(&mut self, epoch: Epoch, value: f64) {
        self.last = epoch;
        self.samples += 1;
        let delta = value - self.mean;
        self.mean += delta / self.samples as f64;
        self.m2 += delta * (value - self.mean);
    }

    fn rms(&self) -> f64 {
        if self.samples > 0 {
            (self.m2 / self.samples as f64).sqrt()
        } else {
            0.0
        }
    }
}

/// Identifies one multipath combination: receiver, satellite, code, other phase
type MultipathKey = (QcIndexing, SV, Observable, Observable);

/// Deterministic order of the multipath combinations
fn sort_key(key: &MultipathKey) -> (String, SV, Observable, Observable) {
    (key.0.to_string(), key.1, key.2.clone(), key.3.clone())
}

/// Signal combinations, for each receiver, satellite and pair of frequencies.
///
/// For each satellite, the signals are sorted per frequency band (RINEX band number). On each
/// band, one code and one phase are selected (civilian first: `C`, then `W`, `P`, `Y`, `X`,
/// `Q`, `I`..). The reference (signal 1) is the lowest band number, usually L1/E1
/// (B1I for BeiDou). Every other band j is combined with it, not only L2 or L5. Values in meters
/// (phases are converted with the wavelength of their carrier):
///
/// | Combination | Phase | Code |
/// |-------------|-------|------|
/// | Geometry Free | `GF = Lj − L1` | `GF = C1 − Cj` |
/// | Ionosphere Free | `IF = (f1² L1 − fj² Lj) / (f1² − fj²)` | `IF = (f1² C1 − fj² Cj) / (f1² − fj²)` |
///
/// Both GF combinations equal `I1 − Ij` (ionospheric delays) plus biases (and ambiguities
/// for the phase): they are directly comparable, the phase GF being much less noisy.
///
/// Code multipath, for each code `Ci` (every code, not only the preferred one) of band i,
/// with the phase `Li` of the same band and the phase `Lj` of another band (the reference band,
/// or the next band for the reference band itself):
///
/// `MPi = Ci − Li − 2 (Li − Lj) / (αij − 1)`, `αij = (fi / fj)²`
///
/// Geometry, clocks, troposphere and ionosphere (first order) cancel out: the code multipath
/// and noise remain, plus a constant bias (phase ambiguities, hardware delays) for each
/// continuous phase arc. An arc ends on a loss of lock (LLI), a data gap or a jump: its
/// statistics are then emitted ([QcMultipathArc]: mean, RMS around the mean).
/// Each multipath value comes with its arc number and its value debiased by the mean
/// of the arc so far.
///
/// Glonass (FDMA) is not supported.
#[derive(Debug, Default, Clone)]
pub struct QcCombinationProcessor {
    cfg: QcCombinationConfig,
    arcs: HashMap<MultipathKey, Arc>,
    next_arc: HashMap<MultipathKey, u32>,
    last_epoch: Option<Epoch>,
}

impl QcCombinationProcessor {
    /// Builds a new [QcCombinationProcessor]
    pub fn new(cfg: QcCombinationConfig) -> Self {
        Self {
            cfg,
            ..Default::default()
        }
    }

    fn close<O: From<QcOutput>>(
        epoch: Epoch,
        key: &MultipathKey,
        arc: &Arc,
        output: &mut QcOutputQueue<O>,
    ) {
        if arc.samples == 0 {
            return;
        }
        output.push(
            epoch,
            QcOutput::MultipathArc(QcMultipathArc {
                receiver: key.0.clone(),
                sv: key.1,
                code: key.2.clone(),
                phase: key.3.clone(),
                arc: arc.id,
                start: arc.start,
                end: arc.last,
                samples: arc.samples,
                mean_m: arc.mean,
                rms_m: arc.rms(),
            }),
        );
    }

    /// Closes the arcs that were not updated for too long (satellite out of sight)
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

    fn multipath_sample<O: From<QcOutput>>(
        &mut self,
        epoch: Epoch,
        key: MultipathKey,
        value: f64,
        lock_loss: bool,
        output: &mut QcOutputQueue<O>,
    ) -> (u32, f64) {
        let restart = match self.arcs.get(&key) {
            Some(arc) => {
                lock_loss
                    || epoch - arc.last > self.cfg.max_gap
                    || (arc.samples > 0 && (value - arc.mean).abs() > self.cfg.jump_threshold_m)
            }
            None => true,
        };

        if restart {
            if let Some(arc) = self.arcs.remove(&key) {
                Self::close(epoch, &key, &arc, output);
            }

            let id = self.next_arc.entry(key.clone()).or_default();
            self.arcs.insert(key.clone(), Arc::new(*id, epoch));
            *id += 1;
        }

        let arc = self.arcs.get_mut(&key).unwrap();
        arc.push(epoch, value);

        (arc.id, value - arc.mean)
    }
}

impl<O: From<QcOutput>> QcProcessor<O> for QcCombinationProcessor {
    fn name(&self) -> &str {
        "combinations"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>) {
        let epoch = ctx.epoch;
        self.last_epoch = Some(epoch);

        let mut emit =
            |kind, receiver: &QcIndexing, sv, lhs: &Observable, rhs: &Observable, value_m| {
                output.push(
                    epoch,
                    QcOutput::Combination(QcCombination {
                        receiver: receiver.clone(),
                        sv,
                        kind,
                        lhs: lhs.clone(),
                        rhs: rhs.clone(),
                        value_m,
                        arc: None,
                        debiased_m: None,
                    }),
                );
            };

        let mut mp_samples = Vec::new();

        for rx in ctx.receivers.iter() {
            if !self.cfg.selected_receiver(&rx.receiver) {
                continue;
            }

            // signals per satellite
            let mut satellites = BTreeMap::<SV, Vec<&SignalObservation>>::new();
            for signal in rx.signals.iter() {
                if self.cfg.selected_constellation(signal.sv.constellation) {
                    satellites.entry(signal.sv).or_default().push(signal);
                }
            }

            for (sv, signals) in satellites.iter() {
                let bands = bands(*sv, signals);

                // phase: reference band and the others
                let phases = bands
                    .iter()
                    .filter_map(|(band, b)| Some((*band, b.frequency_hz, b.phase()?)))
                    .collect::<Vec<_>>();

                if let Some(((_, f1, (l1_obs, l1, _)), others)) = phases.split_first() {
                    for (_, fj, (lj_obs, lj, _)) in others.iter() {
                        if self.cfg.phase && self.cfg.geometry_free {
                            emit(
                                QcCombinationKind::GeometryFree,
                                &rx.receiver,
                                *sv,
                                lj_obs,
                                l1_obs,
                                lj - l1,
                            );
                        }
                        if self.cfg.phase && self.cfg.ionosphere_free {
                            let value = ionosphere_free(*f1, *l1, *fj, *lj);
                            emit(
                                QcCombinationKind::IonosphereFree,
                                &rx.receiver,
                                *sv,
                                l1_obs,
                                lj_obs,
                                value,
                            );
                        }
                    }
                }

                // code: reference band and the others
                let codes = bands
                    .iter()
                    .filter_map(|(band, b)| Some((*band, b.frequency_hz, b.code()?)))
                    .collect::<Vec<_>>();

                if let Some(((_, f1, (c1_obs, c1)), others)) = codes.split_first() {
                    for (_, fj, (cj_obs, cj)) in others.iter() {
                        if self.cfg.code && self.cfg.geometry_free {
                            emit(
                                QcCombinationKind::GeometryFree,
                                &rx.receiver,
                                *sv,
                                c1_obs,
                                cj_obs,
                                c1 - cj,
                            );
                        }
                        if self.cfg.code && self.cfg.ionosphere_free {
                            let value = ionosphere_free(*f1, *c1, *fj, *cj);
                            emit(
                                QcCombinationKind::IonosphereFree,
                                &rx.receiver,
                                *sv,
                                c1_obs,
                                cj_obs,
                                value,
                            );
                        }
                    }
                }

                // code multipath: every code of band i, phases of band i and band j
                if self.cfg.multipath && phases.len() > 1 {
                    for (band, b) in bands.iter() {
                        let Some((_, li, li_lock_loss)) = b.phase() else {
                            continue;
                        };

                        // other band: the reference, or the next one for the reference itself
                        let other = phases.iter().find(|(other, _, _)| other != band).copied();

                        let Some((_, fj, (lj_obs, lj, lj_lock_loss))) = other else {
                            continue;
                        };

                        for (ci_obs, ci) in b.codes.values() {
                            let value = multipath(b.frequency_hz, *ci, *li, fj, *lj);
                            mp_samples.push((
                                (rx.receiver.clone(), *sv, ci_obs.clone(), lj_obs.clone()),
                                value,
                                *li_lock_loss || *lj_lock_loss,
                            ));
                        }
                    }
                }
            }
        }

        for (key, value, lock_loss) in mp_samples {
            let (arc, debiased) =
                self.multipath_sample(epoch, key.clone(), value, lock_loss, output);
            output.push(
                epoch,
                QcOutput::Combination(QcCombination {
                    receiver: key.0,
                    sv: key.1,
                    kind: QcCombinationKind::Multipath,
                    lhs: key.2,
                    rhs: key.3,
                    value_m: value,
                    arc: Some(arc),
                    debiased_m: Some(debiased),
                }),
            );
        }

        self.close_stale_arcs(epoch, output);
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

    const F1: f64 = 1575.42E6;
    const F2: f64 = 1227.60E6;
    const F5: f64 = 1176.45E6;

    /// Synthetic observations: range, ionospheric delay on L1 (m), multipath (m)
    fn observations(rho: f64, iono_l1: f64, mp1: f64, mp2: f64, n1: f64, n2: f64) -> [f64; 4] {
        let i2 = iono_l1 * (F1 / F2).powi(2);
        let (l1_m, l2_m) = (299_792_458.0 / F1, 299_792_458.0 / F2);
        [
            rho + iono_l1 + mp1,       // C1
            rho + i2 + mp2,            // C2
            rho - iono_l1 + n1 * l1_m, // L1 (m)
            rho - i2 + n2 * l2_m,      // L2 (m)
        ]
    }

    #[test]
    fn combinations() {
        let (iono, n1, n2) = (4.2, 1234.0, -987.0);
        let [c1, c2, l1, l2] = observations(22.0E6, iono, 0.0, 0.0, n1, n2);

        let i2 = iono * (F1 / F2).powi(2);

        // GF: I1 - I2 (+ ambiguities for the phase)
        assert!(((c1 - c2) - (iono - i2)).abs() < 1.0E-6);
        let ambiguity = n2 * 299_792_458.0 / F2 - n1 * 299_792_458.0 / F1;
        assert!(((l2 - l1) - (iono - i2 + ambiguity)).abs() < 1.0E-6);

        // IF: the ionosphere cancels out
        assert!((ionosphere_free(F1, c1, F2, c2) - 22.0E6).abs() < 1.0E-6);
        assert!((ionosphere_free(F1, c1, F5, c1 + 0.0) - c1).abs() < 1.0E-6);

        // MP: geometry & ionosphere cancel out, a constant (ambiguities) remains
        let mp_a = multipath(F1, c1, l1, F2, l2);
        let [c1b, _, l1b, l2b] = observations(23.0E6, iono * 2.0, 0.7, 0.0, n1, n2);
        let mp_b = multipath(F1, c1b, l1b, F2, l2b);
        assert!(((mp_b - mp_a) - 0.7).abs() < 1.0E-6, "{}", mp_b - mp_a);

        // MP2, with L1 as other phase
        let mp2_a = multipath(F2, c2, l2, F1, l1);
        let [_, c2b, l1b, l2b] = observations(21.5E6, iono * 3.0, 0.0, -0.4, n1, n2);
        let mp2_b = multipath(F2, c2b, l2b, F1, l1b);
        assert!(((mp2_b - mp2_a) + 0.4).abs() < 1.0E-6, "{}", mp2_b - mp2_a);
    }

    #[test]
    fn attributes() {
        assert!(attribute_rank('C') < attribute_rank('W'));
        assert!(attribute_rank('W') < attribute_rank('X'));
        assert!(attribute_rank('Q') < attribute_rank('?'));
    }
}
