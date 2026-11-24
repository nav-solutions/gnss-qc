//! Synchronization buffers.
//!
//! Context data (ephemerides, precise states..) are delivered by streams that are
//! not aligned with the observations. Each context stream feeds a small FIFO
//! *per satellite* ([QcSvBuffer]), which the engine:
//! - fills ahead of the epoch being processed (lookahead window),
//! - prunes behind the epoch being processed, as soon as data points expire.
//!
//! This keeps the memory footprint bounded (a few data points per satellite)
//! whatever the duration of the session.
use std::collections::{HashMap, VecDeque};

use gnss_rs::prelude::SV;
use hifitime::{Duration, Epoch};

use rinex::navigation::Ephemeris;

use crate::io::{
    input::indexing::QcIndexing,
    source::{QcEphemeris, QcPreciseState},
};

/// Data points that may be buffered in a [QcSvBuffer].
pub trait QcBuffered {
    /// Satellite this data point applies to
    fn sv(&self) -> SV;

    /// Reference epoch of this data point
    fn epoch(&self) -> Epoch;

    /// Returns true if this data point may be used at `t`
    fn is_valid(&self, t: Epoch) -> bool;

    /// Returns true if this data point has expired at `t`
    /// and will never be valid at any `t' >= t`.
    fn has_expired(&self, t: Epoch) -> bool;
}

impl QcBuffered for QcEphemeris {
    fn sv(&self) -> SV {
        self.sv
    }

    fn epoch(&self) -> Epoch {
        self.toc
    }

    fn is_valid(&self, t: Epoch) -> bool {
        QcEphemeris::is_valid(self, t)
    }

    fn has_expired(&self, t: Epoch) -> bool {
        let reference = self.toe().unwrap_or(self.toc);
        let validity = Ephemeris::validity_duration(self.sv.constellation)
            .unwrap_or(Duration::from_hours(4.0));
        t > reference + validity
    }
}

/// Time ordered FIFO per satellite.
#[derive(Debug, Clone)]
pub struct QcSvBuffer<T: QcBuffered> {
    inner: HashMap<SV, VecDeque<T>>,
}

impl<T: QcBuffered> Default for QcSvBuffer<T> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<T: QcBuffered> QcSvBuffer<T> {
    /// Stores a new data point. Sources are time ordered so this is
    /// usually a simple push back; out of order data is inserted at the right place.
    pub fn push(&mut self, item: T) {
        let fifo = self.inner.entry(item.sv()).or_default();
        let epoch = item.epoch();

        match fifo.back() {
            Some(last) if last.epoch() > epoch => {
                let pos = fifo.partition_point(|x| x.epoch() <= epoch);
                fifo.insert(pos, item);
            }
            _ => fifo.push_back(item),
        }
    }

    /// Selects the best data point for this satellite at `t`:
    /// the valid one that is closest in time.
    pub fn select(&self, sv: SV, t: Epoch) -> Option<&T> {
        self.inner
            .get(&sv)?
            .iter()
            .filter(|item| item.is_valid(t))
            .min_by_key(|item| (item.epoch() - t).abs())
    }

    /// Drops all data points that have expired at `t`.
    pub fn prune(&mut self, t: Epoch) {
        self.inner.retain(|_, fifo| {
            fifo.retain(|item| !item.has_expired(t));
            !fifo.is_empty()
        });
    }

    /// Number of buffered data points (all satellites).
    pub fn len(&self) -> usize {
        self.inner.values().map(|fifo| fifo.len()).sum()
    }

    /// Returns true if nothing is buffered.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Time ordered data points of one satellite
    pub fn fifo(&self, sv: SV) -> Option<&VecDeque<T>> {
        self.inner.get(&sv)
    }

    /// Iterates over all buffered data points (all satellites).
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.inner.values().flat_map(|fifo| fifo.iter())
    }

    /// Satellites for which something is buffered.
    pub fn satellites(&self) -> impl Iterator<Item = SV> + '_ {
        self.inner.keys().copied()
    }
}

/// Precise states are kept around the epoch being processed, for interpolation purposes.
/// At the end of the products, the interpolation window only extends backwards:
/// order 10 at 15' sampling requires 2h30 of past data.
pub const PRECISE_STATES_RETENTION: Duration = Duration::from_seconds(3.0 * 3600.0);

impl QcBuffered for QcPreciseState {
    fn sv(&self) -> SV {
        self.sv
    }

    fn epoch(&self) -> Epoch {
        self.epoch
    }

    fn is_valid(&self, t: Epoch) -> bool {
        (t - self.epoch).abs() <= PRECISE_STATES_RETENTION
    }

    fn has_expired(&self, t: Epoch) -> bool {
        t - self.epoch > PRECISE_STATES_RETENTION
    }
}

/// Interpolated precise state
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QcInterpolatedState {
    /// ECEF position (km)
    pub position_km: (f64, f64, f64),

    /// ECEF velocity (km/s)
    pub velocity_km_s: (f64, f64, f64),

    /// Clock offset (s), when both surrounding samples provide one
    pub clock_offset_s: Option<f64>,
}

/// Lagrange interpolation of the samples (time offsets to the interpolation instant, values)
fn lagrange(samples: &[(f64, [f64; 3])], dt: f64) -> [f64; 3] {
    let mut out = [0.0; 3];

    for (i, (t_i, p_i)) in samples.iter().enumerate() {
        let mut l_i = 1.0;
        for (j, (t_j, _)) in samples.iter().enumerate() {
            if i != j {
                l_i *= (dt - t_j) / (t_i - t_j);
            }
        }
        for k in 0..3 {
            out[k] += l_i * p_i[k];
        }
    }

    out
}

/// Interpolates precise states of one satellite at `t`.
/// - order: Lagrange interpolation order (order + 1 samples are used)
/// - no extrapolation: `t` must lie within the samples window
/// - the window must not contain data gaps (> 3 times the smallest sampling interval)
pub fn interpolate_precise_state(
    fifo: &VecDeque<QcPreciseState>,
    t: Epoch,
    order: usize,
) -> Option<QcInterpolatedState> {
    let n = order + 1;

    if fifo.len() < n {
        return None;
    }

    let pos = fifo.partition_point(|s| s.epoch < t);

    if pos == 0 && fifo[0].epoch != t {
        return None; // before first sample
    }

    if pos == fifo.len() {
        return None; // after last sample
    }

    let start = pos.saturating_sub(n / 2).min(fifo.len() - n);
    let window = fifo.range(start..start + n).collect::<Vec<_>>();

    let intervals = window
        .windows(2)
        .map(|w| (w[1].epoch - w[0].epoch).to_seconds())
        .collect::<Vec<_>>();

    let min_interval = intervals.iter().copied().fold(f64::MAX, f64::min);

    if intervals.iter().any(|dt| *dt > 3.0 * min_interval) {
        return None; // data gap
    }

    let samples = window
        .iter()
        .map(|s| {
            (
                (s.epoch - t).to_seconds(),
                [s.position_km.0, s.position_km.1, s.position_km.2],
            )
        })
        .collect::<Vec<_>>();

    let p = lagrange(&samples, 0.0);
    let p_minus = lagrange(&samples, -0.5);
    let p_plus = lagrange(&samples, 0.5);

    // clock: linear interpolation between the two surrounding samples
    let after = &fifo[pos];
    let clock_offset_s = if after.epoch == t {
        after.clock_offset_s
    } else {
        let before = &fifo[pos - 1];
        match (before.clock_offset_s, after.clock_offset_s) {
            (Some(c0), Some(c1)) => {
                let dt = (after.epoch - before.epoch).to_seconds();
                let x = (t - before.epoch).to_seconds() / dt;
                Some(c0 + (c1 - c0) * x)
            }
            _ => None,
        }
    };

    Some(QcInterpolatedState {
        position_km: (p[0], p[1], p[2]),
        velocity_km_s: (
            p_plus[0] - p_minus[0],
            p_plus[1] - p_minus[1],
            p_plus[2] - p_minus[2],
        ),
        clock_offset_s,
    })
}

/// Precise states, buffered per source (agency) and per satellite.
#[derive(Debug, Clone, Default)]
pub struct QcPreciseStore {
    sources: HashMap<QcIndexing, QcSvBuffer<QcPreciseState>>,
}

impl QcPreciseStore {
    /// Default Lagrange interpolation order (SP3: 15' sampling)
    pub const DEFAULT_ORDER: usize = 10;

    /// Stores a new precise state delivered by `source`.
    pub fn push(&mut self, source: &QcIndexing, state: QcPreciseState) {
        if let Some(buffer) = self.sources.get_mut(source) {
            buffer.push(state);
        } else {
            let mut buffer = QcSvBuffer::default();
            buffer.push(state);
            self.sources.insert(source.clone(), buffer);
        }
    }

    /// Interpolates the state of `sv` at `t`, from the first source that can.
    pub fn interpolate(&self, sv: SV, t: Epoch) -> Option<(&QcIndexing, QcInterpolatedState)> {
        let mut sources = self.sources.keys().collect::<Vec<_>>();
        sources.sort_by_key(|s| s.to_string()); // deterministic

        sources
            .into_iter()
            .find_map(|source| Some((source, self.interpolate_from(source, sv, t)?)))
    }

    /// Interpolates the state of `sv` at `t`, from a specific source.
    pub fn interpolate_from(
        &self,
        source: &QcIndexing,
        sv: SV,
        t: Epoch,
    ) -> Option<QcInterpolatedState> {
        let fifo = self.sources.get(source)?.fifo(sv)?;
        interpolate_precise_state(fifo, t, Self::DEFAULT_ORDER)
    }

    /// Samples of `sv` from `source`, time ordered
    pub fn samples(&self, source: &QcIndexing, sv: SV) -> Option<&VecDeque<QcPreciseState>> {
        self.sources.get(source)?.fifo(sv)
    }

    /// Satellites described by this source
    pub fn satellites_of(&self, source: &QcIndexing) -> Vec<SV> {
        self.sources
            .get(source)
            .map(|buffer| buffer.satellites().collect())
            .unwrap_or_default()
    }

    /// Precise states sources that delivered something.
    pub fn sources(&self) -> impl Iterator<Item = &QcIndexing> {
        self.sources.keys()
    }

    /// Drops all expired states.
    pub fn prune(&mut self, t: Epoch) {
        for buffer in self.sources.values_mut() {
            buffer.prune(t);
        }
    }

    /// Total number of buffered states.
    pub fn len(&self) -> usize {
        self.sources.values().map(|b| b.len()).sum()
    }

    /// Returns true if nothing is buffered.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Radio ephemerides, buffered per source (agency, receiver..) and per satellite.
#[derive(Debug, Clone, Default)]
pub struct QcEphemerisStore {
    sources: HashMap<QcIndexing, QcSvBuffer<QcEphemeris>>,
}

impl QcEphemerisStore {
    /// Stores a new ephemeris delivered by `source`.
    pub fn push(&mut self, source: &QcIndexing, ephemeris: QcEphemeris) {
        if let Some(buffer) = self.sources.get_mut(source) {
            buffer.push(ephemeris);
        } else {
            let mut buffer = QcSvBuffer::default();
            buffer.push(ephemeris);
            self.sources.insert(source.clone(), buffer);
        }
    }

    /// Selects the best ephemeris for `sv` at `t`, among all sources.
    pub fn select(&self, sv: SV, t: Epoch) -> Option<(&QcIndexing, &QcEphemeris)> {
        self.sources
            .iter()
            .filter_map(|(source, buffer)| Some((source, buffer.select(sv, t)?)))
            .min_by_key(|(_, eph)| (eph.toc - t).abs())
    }

    /// Selects the best ephemeris for `sv` at `t`, from a specific source.
    /// This is what residual analysis between agencies needs.
    pub fn select_from(&self, source: &QcIndexing, sv: SV, t: Epoch) -> Option<&QcEphemeris> {
        self.sources.get(source)?.select(sv, t)
    }

    /// Iterates over all buffered ephemerides, with their source.
    pub fn iter(&self) -> impl Iterator<Item = (&QcIndexing, &QcEphemeris)> {
        self.sources
            .iter()
            .flat_map(|(source, buffer)| buffer.iter().map(move |eph| (source, eph)))
    }

    /// Satellites described by this source
    pub fn satellites_of(&self, source: &QcIndexing) -> Vec<SV> {
        self.sources
            .get(source)
            .map(|buffer| buffer.satellites().collect())
            .unwrap_or_default()
    }

    /// Ephemeris sources that delivered something.
    pub fn sources(&self) -> impl Iterator<Item = &QcIndexing> {
        self.sources.keys()
    }

    /// Drops all expired ephemerides.
    pub fn prune(&mut self, t: Epoch) {
        for buffer in self.sources.values_mut() {
            buffer.prune(t);
        }
    }

    /// Total number of buffered ephemerides.
    pub fn len(&self) -> usize {
        self.sources.values().map(|b| b.len()).sum()
    }

    /// Returns true if nothing is buffered.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tests::toolkit::gps_ephemeris;
    use std::str::FromStr;

    #[test]
    fn sv_buffer_select_and_prune() {
        let g01 = SV::from_str("G01").unwrap();
        let t0 = Epoch::from_str("2020-06-25T00:00:00 GPST").unwrap();

        let mut buffer = QcSvBuffer::<QcEphemeris>::default();

        // out of order on purpose
        buffer.push(gps_ephemeris(g01, t0 + Duration::from_hours(2.0)));
        buffer.push(gps_ephemeris(g01, t0));
        buffer.push(gps_ephemeris(g01, t0 + Duration::from_hours(4.0)));
        assert_eq!(buffer.len(), 3);

        let t = t0 + Duration::from_seconds(50.0 * 60.0);
        assert_eq!(buffer.select(g01, t).unwrap().toc, t0);

        let t = t0 + Duration::from_seconds(70.0 * 60.0);
        assert_eq!(
            buffer.select(g01, t).unwrap().toc,
            t0 + Duration::from_hours(2.0)
        );

        // unknown SV
        assert!(buffer.select(SV::from_str("G02").unwrap(), t).is_none());

        // t0 ephemeris expires after t0 + 2h
        buffer.prune(t0 + Duration::from_hours(2.5));
        assert_eq!(buffer.len(), 2);

        buffer.prune(t0 + Duration::from_hours(10.0));
        assert!(buffer.is_empty());
    }

    #[test]
    fn precise_state_interpolation() {
        let g01 = SV::from_str("G01").unwrap();
        let t0 = Epoch::from_str("2020-06-25T00:00:00 GPST").unwrap();
        let dt = Duration::from_seconds(900.0);

        // circular orbit, 12h period, 26560 km radius; linear clock
        let w = 2.0 * std::f64::consts::PI / 43200.0;
        let r = 26560.0;
        let truth = |t: f64| (r * (w * t).cos(), r * (w * t).sin(), 0.0);

        let mut store = QcPreciseStore::default();
        let agency = QcIndexing::Agency("TEST".to_string());

        for k in 0..24 {
            let t = k as f64 * 900.0;
            store.push(
                &agency,
                QcPreciseState {
                    sv: g01,
                    epoch: t0 + dt * k,
                    position_km: truth(t),
                    clock_offset_s: Some(1.0E-4 + 1.0E-9 * t),
                },
            );
        }

        let t = 3.0 * 3600.0 + 123.4;
        let state = store
            .interpolate(g01, t0 + Duration::from_seconds(t))
            .unwrap()
            .1;

        let expected = truth(t);
        assert!((state.position_km.0 - expected.0).abs() < 1.0E-6);
        assert!((state.position_km.1 - expected.1).abs() < 1.0E-6);

        // |v| = w.r
        let v = (state.velocity_km_s.0.powi(2) + state.velocity_km_s.1.powi(2)).sqrt();
        assert!((v - w * r).abs() < 1.0E-6);

        let clock = state.clock_offset_s.unwrap();
        assert!((clock - (1.0E-4 + 1.0E-9 * t)).abs() < 1.0E-15);

        // no extrapolation
        assert!(store
            .interpolate(g01, t0 - Duration::from_seconds(1.0))
            .is_none());
        assert!(store.interpolate(g01, t0 + dt * 24).is_none());

        // exact sample
        let state = store.interpolate(g01, t0 + dt * 5).unwrap().1;
        assert!((state.position_km.0 - truth(4500.0).0).abs() < 1.0E-9);
    }
}
