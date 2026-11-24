//! Satellite orbits, from the synchronized context.
//!
//! Satellites states are resolved from either:
//! - precise products (SP3): Lagrange interpolation of the precise states,
//! - broadcast ephemerides (NAV): Keplerian solving ([QcKepler]).
//!
//! [QcOrbitPreference] defines how both are combined. These orbits are shared by
//! the navigation solver and the other processors (ionospheric pierce points..).
mod kepler;

pub use kepler::{relativistic_correction_s, QcKepler, QcKeplerState, SPEED_OF_LIGHT_M_S};

use gnss_rs::prelude::SV;
use hifitime::Epoch;

use crate::{
    engine::QcEpochContext,
    sync::{interpolate_precise_state, QcPreciseStore},
};

/// Orbital products preference.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QcOrbitPreference {
    /// Precise products (SP3) whenever they cover the satellite at the epoch being
    /// processed (position and clock), broadcast ephemerides otherwise. Default.
    #[default]
    Auto,

    /// Precise products only: satellites without precise products are not used.
    PreciseOnly,

    /// Broadcast ephemerides only: precise products are disregarded.
    BroadcastOnly,
}

/// Kind of orbit used for one satellite, at one epoch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QcOrbitKind {
    /// Precise products (SP3)
    Precise,
    /// Broadcast ephemerides (NAV)
    Broadcast,
}

/// Satellite state resolved at one instant
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QcSatelliteState {
    /// Product this state was resolved from
    pub kind: QcOrbitKind,
    /// ECEF position (m)
    pub position_m: (f64, f64, f64),
    /// ECEF velocity (m/s)
    pub velocity_m_s: (f64, f64, f64),
}

/// Resolves the state of `sv` at `t` from the synchronized context, according to
/// the [QcOrbitPreference]. Precise products are never extrapolated.
pub fn satellite_state(
    ctx: &QcEpochContext,
    sv: SV,
    t: Epoch,
    preference: QcOrbitPreference,
) -> Option<QcSatelliteState> {
    if preference != QcOrbitPreference::BroadcastOnly {
        if let Some((_, state)) = ctx.precise_states.interpolate(sv, t) {
            let (x, y, z) = state.position_km;
            let (vx, vy, vz) = state.velocity_km_s;
            return Some(QcSatelliteState {
                kind: QcOrbitKind::Precise,
                position_m: (x * 1.0E3, y * 1.0E3, z * 1.0E3),
                velocity_m_s: (vx * 1.0E3, vy * 1.0E3, vz * 1.0E3),
            });
        }
    }

    if preference != QcOrbitPreference::PreciseOnly {
        let (_, eph) = ctx.ephemerides.select(sv, t)?;
        let kepler = QcKepler::from_ephemeris(sv, eph.toc, &eph.ephemeris)?;
        let state = kepler.state(t);
        return Some(QcSatelliteState {
            kind: QcOrbitKind::Broadcast,
            position_m: state.position_m,
            velocity_m_s: state.velocity_m_s,
        });
    }

    None
}

/// Returns true if the precise products of this store cover `sv` at `t`
pub fn precise_coverage(store: &QcPreciseStore, sv: SV, t: Epoch) -> bool {
    store.sources().any(|source| {
        store
            .samples(source, sv)
            .and_then(|fifo| interpolate_precise_state(fifo, t, QcPreciseStore::DEFAULT_ORDER))
            .is_some()
    })
}
