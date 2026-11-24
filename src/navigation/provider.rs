//! Navigation data provider: implements the GNSS-RTK data traits
//! on top of the engine synchronized context.
//!
//! GNSS-RTK pulls orbital states and clock corrections *during* the resolution,
//! at instants it determines itself (transmission time). Before each resolution,
//! the processor takes a small snapshot of the engine context (per satellite):
//! - the radio ephemeris selected for this epoch (Keplerian parameters, Copy),
//! - the precise states window around this epoch (a few samples),
//!
//! - the precise clock states window around this epoch (Clock RINEX, a few samples),
//!
//! and decides, per satellite, which kind of orbit ([QcOrbitPreference]) and which
//! kind of clock ([QcClockPreference]) are used. By default, the clock follows the
//! orbit (consistent products), precise clock products being preferred to the clock
//! states of the precise products.
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use gnss_rtk::prelude::{
    AbsoluteTime, BiasRuntime, Duration, EnvironmentalBias, Ephemeris as RtkEphemeris,
    EphemerisSource, Epoch, Frame, Method, Orbit, OrbitSource, SatelliteClockCorrection,
    SpacebornBias, TimeScale, TroposphereModel, SV,
};

use crate::{
    engine::QcEpochContext,
    io::{
        input::indexing::QcIndexing,
        source::{QcClockId, QcPreciseState},
    },
    orbit::{
        relativistic_correction_s, QcClockKind, QcClockPreference, QcKepler, QcOrbitKind,
        QcOrbitPreference,
    },
    sync::{interpolate_linear, interpolate_precise_state, QcClockStore, QcPreciseStore},
};

type Vector3D = (f64, f64, f64);

/// Upper bound of the signal time of flight (GEO satellites ~0.13 s)
const MAX_TIME_OF_FLIGHT: Duration = Duration::from_milliseconds(150.0);

#[derive(Debug, Default, Clone)]
struct SvData {
    kind: Option<QcOrbitKind>,
    clock: Option<QcClockKind>,
    kepler: Option<QcKepler>,
    precise: VecDeque<QcPreciseState>,
    /// Precise clock products samples around the epoch (epoch, offset s)
    clocks: VecDeque<(Epoch, f64)>,
}

/// Counters of orbital states and clock states served to the solver
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct QcOrbitCounters {
    pub precise: usize,
    pub broadcast: usize,
    pub precise_clocks: usize,
    pub precise_state_clocks: usize,
    pub broadcast_clocks: usize,
}

/// Navigation data snapshot, shared between the GNSS-RTK traits implementations.
#[derive(Debug, Default)]
pub struct QcNavigationData {
    preference: QcOrbitPreference,
    clock_preference: QcClockPreference,
    method: Method,
    svs: HashMap<SV, SvData>,
    precise_orbits: Cell<usize>,
    broadcast_orbits: Cell<usize>,
    precise_clocks: Cell<usize>,
    precise_state_clocks: Cell<usize>,
    broadcast_clocks: Cell<usize>,
}

impl QcNavigationData {
    pub fn new(
        preference: QcOrbitPreference,
        clock_preference: QcClockPreference,
        method: Method,
    ) -> Self {
        Self {
            preference,
            clock_preference,
            method,
            ..Default::default()
        }
    }

    /// Orbital and clock states served so far
    pub fn counters(&self) -> QcOrbitCounters {
        QcOrbitCounters {
            precise: self.precise_orbits.get(),
            broadcast: self.broadcast_orbits.get(),
            precise_clocks: self.precise_clocks.get(),
            precise_state_clocks: self.precise_state_clocks.get(),
            broadcast_clocks: self.broadcast_clocks.get(),
        }
    }

    /// Selected orbit kind for this satellite, at the current epoch
    pub fn orbit_kind(&self, sv: SV) -> Option<QcOrbitKind> {
        self.svs.get(&sv)?.kind
    }

    /// Selected clock kind for this satellite, at the current epoch
    pub fn clock_kind(&self, sv: SV) -> Option<QcClockKind> {
        let data = self.svs.get(&sv)?;
        data.kind?;
        data.clock
    }

    /// Updates the snapshot for these satellites, at the epoch being processed.
    pub fn update<I: Iterator<Item = SV>>(&mut self, ctx: &QcEpochContext, svs: I) {
        let t = ctx.epoch;
        let t_tx = t - MAX_TIME_OF_FLIGHT;

        for data in self.svs.values_mut() {
            data.kind = None;
            data.clock = None;
        }

        for sv in svs {
            let data = self.svs.entry(sv).or_default();

            // precise products: window of the first source able to interpolate
            data.precise.clear();

            if self.preference != QcOrbitPreference::BroadcastOnly
                || self.clock_preference != QcClockPreference::Broadcast
            {
                if let Some(samples) = Self::precise_window(ctx.precise_states, sv, t) {
                    data.precise.extend(samples.iter().copied());
                }
            }

            // precise clock products: samples around this epoch
            data.clocks.clear();

            if self.clock_preference != QcClockPreference::Broadcast {
                if let Some(samples) = Self::clock_window(ctx.precise_clocks, sv, t) {
                    data.clocks.extend(samples);
                }
            }

            // broadcast: refresh only when the selected ephemeris changes.
            // Always loaded (even PreciseOnly): it also provides the group delays.
            match ctx.ephemerides.select(sv, t) {
                Some((_, eph)) => {
                    if data.kepler.map(|k| k.toc) != Some(eph.toc) {
                        data.kepler = QcKepler::from_ephemeris(sv, eph.toc, &eph.ephemeris);
                    }
                }
                None => data.kepler = None,
            }

            // precise products are usable when they interpolate at the sampling instant
            // and at the transmission instant (~70 ms earlier): no extrapolation
            // (beginning of a file for example).
            let precise_at = |t: Epoch| {
                interpolate_precise_state(&data.precise, t, QcPreciseStore::DEFAULT_ORDER)
            };

            let (precise_orbit, precise_state_clock) = match (precise_at(t), precise_at(t_tx)) {
                (Some(s0), Some(s1)) => (
                    true,
                    s0.clock_offset_s.is_some() && s1.clock_offset_s.is_some(),
                ),
                _ => (false, false),
            };

            let precise_clock = interpolate_linear(&data.clocks, t, |s| *s).is_some()
                && interpolate_linear(&data.clocks, t_tx, |s| *s).is_some();

            let broadcast = data.kepler.is_some();

            // best precise clock
            let precise_clock_kind = if precise_clock {
                Some(QcClockKind::PreciseClock)
            } else if precise_state_clock {
                Some(QcClockKind::PreciseState)
            } else {
                None
            };

            // orbit selection
            data.kind = match self.preference {
                QcOrbitPreference::Auto => {
                    // with clocks following the orbit, a precise orbit requires
                    // a precise clock (consistent products)
                    let precise_ok = precise_orbit
                        && (self.clock_preference != QcClockPreference::Auto
                            || precise_clock_kind.is_some());

                    if precise_ok {
                        Some(QcOrbitKind::Precise)
                    } else if broadcast {
                        Some(QcOrbitKind::Broadcast)
                    } else {
                        None
                    }
                }
                QcOrbitPreference::PreciseOnly => precise_orbit.then_some(QcOrbitKind::Precise),
                QcOrbitPreference::BroadcastOnly => broadcast.then_some(QcOrbitKind::Broadcast),
            };

            // clock selection
            data.clock = match (self.clock_preference, data.kind) {
                (_, None) => None,
                (QcClockPreference::Auto, Some(QcOrbitKind::Precise)) => precise_clock_kind,
                (QcClockPreference::Auto, Some(QcOrbitKind::Broadcast)) => {
                    broadcast.then_some(QcClockKind::Broadcast)
                }
                (QcClockPreference::Precise, _) => precise_clock_kind,
                (QcClockPreference::Broadcast, _) => broadcast.then_some(QcClockKind::Broadcast),
            };

            // a satellite requires both an orbit and a clock
            if data.clock.is_none() {
                data.kind = None;
            }
        }
    }

    /// Precise clock products samples of `sv` around `t` (epoch, offset s),
    /// from the first source able to interpolate (sources sorted by name).
    fn clock_window(store: &QcClockStore, sv: SV, t: Epoch) -> Option<Vec<(Epoch, f64)>> {
        let clock = QcClockId::Satellite(sv);

        let mut sources = store.sources().collect::<Vec<&QcIndexing>>();
        sources.sort_by_key(|s| s.to_string());

        sources.into_iter().find_map(|source| {
            store.interpolate_from(source, &clock, t)?;
            let fifo = store.samples(source, &clock)?;

            // enough samples to interpolate at t and t - time of flight,
            // with the data gap verification
            let pos = fifo.partition_point(|c| c.epoch < t);
            let start = pos.saturating_sub(3);
            let end = (pos + 3).min(fifo.len());

            Some(
                fifo.range(start..end)
                    .map(|c| (c.epoch, c.offset_s))
                    .collect(),
            )
        })
    }

    fn precise_window(
        store: &QcPreciseStore,
        sv: SV,
        t: Epoch,
    ) -> Option<&VecDeque<QcPreciseState>> {
        let mut sources = store.sources().collect::<Vec<_>>();
        sources.sort_by_key(|s| s.to_string());

        sources.into_iter().find_map(|source| {
            store.interpolate_from(source, sv, t)?;
            store.samples(source, sv)
        })
    }

    /// Satellite (position m, velocity m/s) at `t`, from the selected product
    fn state_m(&self, sv: SV, t: Epoch) -> Option<(Vector3D, Vector3D)> {
        let data = self.svs.get(&sv)?;

        match data.kind? {
            QcOrbitKind::Precise => {
                let state =
                    interpolate_precise_state(&data.precise, t, QcPreciseStore::DEFAULT_ORDER)?;
                let (x, y, z) = state.position_km;
                let (vx, vy, vz) = state.velocity_km_s;
                Some((
                    (x * 1.0E3, y * 1.0E3, z * 1.0E3),
                    (vx * 1.0E3, vy * 1.0E3, vz * 1.0E3),
                ))
            }
            QcOrbitKind::Broadcast => {
                let state = data.kepler?.state(t);
                Some((state.position_m, state.velocity_m_s))
            }
        }
    }

    /// Satellite clock offset (s) at `t`, relativistic correction included,
    /// from the selected clock product.
    pub fn clock_s(&self, sv: SV, t: Epoch) -> Option<f64> {
        let data = self.svs.get(&sv)?;
        data.kind?;

        match data.clock? {
            QcClockKind::PreciseClock => {
                let offset_s = interpolate_linear(&data.clocks, t, |s| *s)?;
                let (p, v) = self.state_m(sv, t)?;
                Some(offset_s + relativistic_correction_s(p, v))
            }
            QcClockKind::PreciseState => {
                let state =
                    interpolate_precise_state(&data.precise, t, QcPreciseStore::DEFAULT_ORDER)?;
                let (p, v) = self.state_m(sv, t)?;
                Some(state.clock_offset_s? + relativistic_correction_s(p, v))
            }
            QcClockKind::Broadcast => {
                let kepler = data.kepler?;
                let state = kepler.state(t);
                Some(
                    kepler.clock_offset_s(t)
                        + kepler.relativistic_correction_s(state.eccentric_anomaly),
                )
            }
        }
    }

    /// Group delay (s) to apply to the primary signal (single frequency navigation).
    /// Dual frequency methods: the iono free combination is the clock reference
    /// of both broadcast and precise products, no correction applies.
    fn group_delay_s(&self, sv: SV) -> f64 {
        if self.method != Method::SPP {
            return 0.0;
        }

        let data = match self.svs.get(&sv) {
            Some(data) => data,
            None => return 0.0,
        };

        // Both broadcast and precise clocks are referenced to the iono free combination:
        // the primary signal requires the broadcast group delay, whatever the product.
        match (data.kind, data.kepler) {
            (Some(_), Some(kepler)) => -kepler.tgd_s.unwrap_or_default(),
            _ => 0.0,
        }
    }
}

/// [OrbitSource] implementation
pub struct QcOrbitProvider {
    pub data: Rc<RefCell<QcNavigationData>>,
}

impl OrbitSource for QcOrbitProvider {
    fn state_at(&self, epoch: Epoch, sv: SV, frame: Frame) -> Option<Orbit> {
        let data = self.data.borrow();
        let ((x, y, z), _) = data.state_m(sv, epoch)?;

        match data.orbit_kind(sv)? {
            QcOrbitKind::Precise => data.precise_orbits.set(data.precise_orbits.get() + 1),
            QcOrbitKind::Broadcast => data.broadcast_orbits.set(data.broadcast_orbits.get() + 1),
        }

        Some(Orbit::from_position(
            x / 1.0E3,
            y / 1.0E3,
            z / 1.0E3,
            epoch,
            frame,
        ))
    }
}

/// [SpacebornBias] implementation (satellite clocks and group delays)
pub struct QcClockProvider {
    pub data: Rc<RefCell<QcNavigationData>>,
}

impl SpacebornBias for QcClockProvider {
    fn clock_bias(&self, rtm: &BiasRuntime) -> SatelliteClockCorrection {
        let data = self.data.borrow();

        match data.clock_s(rtm.sv, rtm.epoch) {
            Some(clock_s) => {
                let counter = match data.clock_kind(rtm.sv) {
                    Some(QcClockKind::PreciseClock) => &data.precise_clocks,
                    Some(QcClockKind::PreciseState) => &data.precise_state_clocks,
                    _ => &data.broadcast_clocks,
                };

                counter.set(counter.get() + 1);
                SatelliteClockCorrection::with_relativistic_correction_s(clock_s)
            }
            None => SatelliteClockCorrection::default(),
        }
    }

    fn group_delay(&self, rtm: &BiasRuntime) -> Duration {
        Duration::from_seconds(self.data.borrow().group_delay_s(rtm.sv))
    }

    fn mw_bias(&self, _: &BiasRuntime) -> f64 {
        0.0
    }
}

/// [EnvironmentalBias] implementation: Niell troposphere model.
/// Ionosphere: no model yet (dual frequency methods cancel it).
#[derive(Debug, Default, Clone, Copy)]
pub struct QcEnvironment;

impl EnvironmentalBias for QcEnvironment {
    fn troposphere_bias_m(&self, rtm: &BiasRuntime) -> f64 {
        TroposphereModel::Niel.bias_m(rtm)
    }

    fn ionosphere_bias_m(&self, _: &BiasRuntime) -> f64 {
        0.0
    }
}

/// [AbsoluteTime] implementation: [Epoch::to_time_scale].
#[derive(Debug, Default, Clone, Copy)]
pub struct QcTime;

impl AbsoluteTime for QcTime {
    fn new_epoch(&mut self, _: Epoch) {}

    fn epoch_correction(&self, t: Epoch, target: TimeScale) -> Epoch {
        t.to_time_scale(target)
    }
}

/// [EphemerisSource] implementation: orbits are always resolved by gnss-qc
/// (see [QcOrbitProvider]), GNSS-RTK internal Keplerian solving is not used.
#[derive(Debug, Default, Clone, Copy)]
pub struct QcNullEphemeris;

impl EphemerisSource for QcNullEphemeris {
    fn ephemeris_data(&self, _: Epoch, _: SV) -> Option<RtkEphemeris> {
        None
    }
}
