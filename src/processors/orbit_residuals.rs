//! Orbit residuals: A - B between two orbital products (precise or broadcast,
//! from given agencies or sources).
use gnss_rs::prelude::{Constellation, SV};
use hifitime::{Duration, Epoch};

use crate::{
    engine::{QcEpochContext, QcProcessor},
    io::{
        input::indexing::QcIndexing,
        output::{QcOrbitResidual, QcOutput, QcOutputQueue},
    },
    orbit::{QcKepler, QcOrbitKind},
};

/// One side of an orbit residual
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QcOrbitProduct {
    /// Precise products (SP3) of this agency, or of any agency (None)
    Precise(Option<QcIndexing>),

    /// Broadcast ephemerides (NAV) of this source, or of any source (None)
    Broadcast(Option<QcIndexing>),
}

/// Orbit residuals configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QcOrbitResidualConfig {
    /// Product A (minuend)
    pub lhs: QcOrbitProduct,

    /// Product B (subtrahend)
    pub rhs: QcOrbitProduct,

    /// Minimal interval between two evaluations. Default: every epoch
    /// (for example, every precise product sample in a context only session).
    pub sampling: Option<Duration>,

    /// Constellations. Default: all constellations.
    pub constellations: Option<Vec<Constellation>>,

    /// Broadcast ephemerides: maximal distance between the evaluation instant and
    /// the time of clock. Default: the validity period of the constellation.
    /// Single station navigation files only provide distant ephemerides for the
    /// satellites out of sight: restricting to recent ephemerides (1 hour for example)
    /// avoids characterizing extrapolated orbits.
    pub max_broadcast_age: Option<Duration>,
}

impl Default for QcOrbitResidualConfig {
    /// Broadcast ephemerides - precise products
    fn default() -> Self {
        Self::broadcast_precise()
    }
}

impl QcOrbitResidualConfig {
    fn new(lhs: QcOrbitProduct, rhs: QcOrbitProduct) -> Self {
        Self {
            lhs,
            rhs,
            sampling: None,
            constellations: None,
            max_broadcast_age: None,
        }
    }

    /// Precise products of two agencies: A - B
    pub fn precise(lhs: QcIndexing, rhs: QcIndexing) -> Self {
        Self::new(
            QcOrbitProduct::Precise(Some(lhs)),
            QcOrbitProduct::Precise(Some(rhs)),
        )
    }

    /// Broadcast ephemerides (any source) - precise products (any agency)
    pub fn broadcast_precise() -> Self {
        Self::new(
            QcOrbitProduct::Broadcast(None),
            QcOrbitProduct::Precise(None),
        )
    }

    /// Broadcast ephemerides of two sources: A - B
    pub fn broadcast(lhs: QcIndexing, rhs: QcIndexing) -> Self {
        Self::new(
            QcOrbitProduct::Broadcast(Some(lhs)),
            QcOrbitProduct::Broadcast(Some(rhs)),
        )
    }

    /// Minimal interval between two evaluations
    pub fn with_sampling(mut self, sampling: Duration) -> Self {
        self.sampling = Some(sampling);
        self
    }

    /// Broadcast ephemerides: maximal distance to their time of clock
    pub fn with_max_broadcast_age(mut self, age: Duration) -> Self {
        self.max_broadcast_age = Some(age);
        self
    }

    /// Restricts to these constellations
    pub fn with_constellations(mut self, constellations: &[Constellation]) -> Self {
        self.constellations = Some(constellations.to_vec());
        self
    }
}

/// Satellite state resolved from one product
struct State {
    source: QcIndexing,
    kind: QcOrbitKind,
    toc: Option<Epoch>,
    position_m: (f64, f64, f64),
    velocity_m_s: (f64, f64, f64),
    clock_s: Option<f64>,
}

/// Resolves the state of `sv` at `t` from this product
fn resolve(
    ctx: &QcEpochContext,
    product: &QcOrbitProduct,
    sv: SV,
    t: Epoch,
    max_age: Option<Duration>,
) -> Option<State> {
    match product {
        QcOrbitProduct::Precise(source) => {
            let (source, state) = match source {
                Some(source) => (
                    source.clone(),
                    ctx.precise_states.interpolate_from(source, sv, t)?,
                ),
                None => {
                    let (source, state) = ctx.precise_states.interpolate(sv, t)?;
                    (source.clone(), state)
                }
            };

            let (x, y, z) = state.position_km;
            let (vx, vy, vz) = state.velocity_km_s;

            Some(State {
                source,
                kind: QcOrbitKind::Precise,
                toc: None,
                position_m: (x * 1.0E3, y * 1.0E3, z * 1.0E3),
                velocity_m_s: (vx * 1.0E3, vy * 1.0E3, vz * 1.0E3),
                clock_s: state.clock_offset_s,
            })
        }
        QcOrbitProduct::Broadcast(source) => {
            let (source, eph) = match source {
                Some(source) => (source.clone(), ctx.ephemerides.select_from(source, sv, t)?),
                None => {
                    let (source, eph) = ctx.ephemerides.select(sv, t)?;
                    (source.clone(), eph)
                }
            };

            if max_age.is_some_and(|age| (t - eph.toc).abs() > age) {
                return None;
            }

            let kepler = QcKepler::from_ephemeris(sv, eph.toc, &eph.ephemeris)?;
            let state = kepler.state(t);

            Some(State {
                source,
                kind: QcOrbitKind::Broadcast,
                toc: Some(eph.toc),
                position_m: state.position_m,
                velocity_m_s: state.velocity_m_s,
                // same definition as the precise clocks: no relativistic correction
                clock_s: Some(kepler.clock_offset_s(t)),
            })
        }
    }
}

/// Satellites described by this product at this epoch
fn satellites(ctx: &QcEpochContext, product: &QcOrbitProduct) -> Vec<SV> {
    let mut svs = match product {
        QcOrbitProduct::Precise(Some(source)) => ctx.precise_states.satellites_of(source),
        QcOrbitProduct::Precise(None) => ctx
            .precise_states
            .sources()
            .flat_map(|source| ctx.precise_states.satellites_of(source))
            .collect(),
        QcOrbitProduct::Broadcast(Some(source)) => ctx.ephemerides.satellites_of(source),
        QcOrbitProduct::Broadcast(None) => ctx
            .ephemerides
            .sources()
            .flat_map(|source| ctx.ephemerides.satellites_of(source))
            .collect(),
    };

    svs.sort();
    svs.dedup();
    svs
}

fn cross(a: (f64, f64, f64), b: (f64, f64, f64)) -> (f64, f64, f64) {
    (
        a.1 * b.2 - a.2 * b.1,
        a.2 * b.0 - a.0 * b.2,
        a.0 * b.1 - a.1 * b.0,
    )
}

fn dot(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
    a.0 * b.0 + a.1 * b.1 + a.2 * b.2
}

fn unit(a: (f64, f64, f64)) -> (f64, f64, f64) {
    let norm = dot(a, a).sqrt();
    (a.0 / norm, a.1 / norm, a.2 / norm)
}

/// Orbit residuals between two orbital products: for each satellite described by both
/// products, at each epoch (or at the requested sampling), the satellite states are resolved
/// from both products (Lagrange interpolation of the precise products, Keplerian solving
/// of the broadcast ephemerides) and differentiated: A - B, in ECEF and in the
/// radial / along track / cross track frame of A, plus the clock residual.
#[derive(Debug, Clone)]
pub struct QcOrbitResidualProcessor {
    cfg: QcOrbitResidualConfig,
    last: Option<Epoch>,
}

impl Default for QcOrbitResidualProcessor {
    fn default() -> Self {
        Self::new(QcOrbitResidualConfig::default())
    }
}

impl QcOrbitResidualProcessor {
    /// Builds a new [QcOrbitResidualProcessor]
    pub fn new(cfg: QcOrbitResidualConfig) -> Self {
        Self { cfg, last: None }
    }
}

impl<O: From<QcOutput>> QcProcessor<O> for QcOrbitResidualProcessor {
    fn name(&self) -> &str {
        "orbit-residuals"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>) {
        let t = ctx.epoch;

        if let (Some(sampling), Some(last)) = (self.cfg.sampling, self.last) {
            if t - last < sampling {
                return;
            }
        }

        let mut evaluated = false;

        for sv in satellites(ctx, &self.cfg.lhs) {
            if let Some(constellations) = &self.cfg.constellations {
                if !constellations.contains(&sv.constellation) {
                    continue;
                }
            }

            let (lhs, rhs) = match (
                resolve(ctx, &self.cfg.lhs, sv, t, self.cfg.max_broadcast_age),
                resolve(ctx, &self.cfg.rhs, sv, t, self.cfg.max_broadcast_age),
            ) {
                (Some(lhs), Some(rhs)) => (lhs, rhs),
                _ => continue,
            };

            // the same source on both sides makes no sense
            if lhs.kind == rhs.kind && lhs.source == rhs.source {
                continue;
            }

            let ecef_m = (
                lhs.position_m.0 - rhs.position_m.0,
                lhs.position_m.1 - rhs.position_m.1,
                lhs.position_m.2 - rhs.position_m.2,
            );

            // radial, along track, cross track (frame of A)
            let radial = unit(lhs.position_m);
            let cross_track = unit(cross(lhs.position_m, lhs.velocity_m_s));
            let along_track = cross(cross_track, radial);

            output.push(
                t,
                QcOutput::OrbitResidual(QcOrbitResidual {
                    sv,
                    lhs_source: lhs.source,
                    lhs_kind: lhs.kind,
                    lhs_toc: lhs.toc,
                    rhs_source: rhs.source,
                    rhs_kind: rhs.kind,
                    rhs_toc: rhs.toc,
                    ecef_m,
                    radial_m: dot(ecef_m, radial),
                    along_track_m: dot(ecef_m, along_track),
                    cross_track_m: dot(ecef_m, cross_track),
                    clock_s: match (lhs.clock_s, rhs.clock_s) {
                        (Some(a), Some(b)) => Some(a - b),
                        _ => None,
                    },
                }),
            );

            evaluated = true;
        }

        if evaluated {
            self.last = Some(t);
        }
    }
}
