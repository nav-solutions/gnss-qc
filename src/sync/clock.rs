//! Precise clocks and meteo observations buffers.
use std::collections::{HashMap, VecDeque};

use gnss_rs::prelude::SV;
use hifitime::{Duration, Epoch};

use rinex::{clock::ClockProfileType, prelude::Observable};

use crate::io::{
    input::indexing::QcIndexing,
    source::{QcClockId, QcMeteoSample, QcPreciseClock},
};

/// Precise clock states are kept this long behind the epoch being processed.
pub const PRECISE_CLOCKS_RETENTION: Duration = Duration::from_seconds(3600.0);

/// Meteo observations are kept this long behind the epoch being processed.
pub const METEO_RETENTION: Duration = Duration::from_seconds(3.0 * 3600.0);

/// Linear interpolation of a time ordered series at `t`:
/// - exact sample: returned as is
/// - no extrapolation: `t` must lie between two samples
/// - no interpolation across data gaps: the interval surrounding `t` must not exceed
///   3 times the neighboring sampling intervals.
pub fn interpolate_linear<T>(
    series: &VecDeque<T>,
    t: Epoch,
    sample: impl Fn(&T) -> (Epoch, f64),
) -> Option<f64> {
    let pos = series.partition_point(|s| sample(s).0 < t);

    if pos == series.len() {
        return None; // after last sample
    }

    let (t1, y1) = sample(&series[pos]);

    if t1 == t {
        return Some(y1);
    }

    if pos == 0 {
        return None; // before first sample
    }

    let (t0, y0) = sample(&series[pos - 1]);
    let dt = (t1 - t0).to_seconds();

    // neighboring sampling intervals
    let mut reference = f64::MAX;

    if pos >= 2 {
        reference = reference.min((t0 - sample(&series[pos - 2]).0).to_seconds());
    }

    if pos + 1 < series.len() {
        reference = reference.min((sample(&series[pos + 1]).0 - t1).to_seconds());
    }

    if reference < f64::MAX && dt > 3.0 * reference {
        return None; // data gap
    }

    let x = (t - t0).to_seconds() / dt;
    Some(y0 + (y1 - y0) * x)
}

/// Precise clock states, buffered per source (agency) and per clock.
/// Only analysis results are buffered: satellite clocks (`AS`) and
/// ground station clocks (`AR`).
#[derive(Debug, Default, Clone)]
pub struct QcClockStore {
    sources: HashMap<QcIndexing, HashMap<QcClockId, VecDeque<QcPreciseClock>>>,
}

impl QcClockStore {
    /// Stores a new clock state delivered by `source`.
    pub fn push(&mut self, source: &QcIndexing, clock: QcPreciseClock) {
        let analysis = match &clock.clock {
            QcClockId::Satellite(_) => clock.profile == ClockProfileType::AS,
            QcClockId::Station(_) => clock.profile == ClockProfileType::AR,
        };

        if !analysis {
            return;
        }

        let fifo = self
            .sources
            .entry(source.clone())
            .or_default()
            .entry(clock.clock.clone())
            .or_default();

        match fifo.back() {
            Some(last) if last.epoch > clock.epoch => {
                let pos = fifo.partition_point(|c| c.epoch <= clock.epoch);
                fifo.insert(pos, clock);
            }
            _ => fifo.push_back(clock),
        }
    }

    /// Clock offset (s) of `clock` at `t`, linearly interpolated,
    /// from the first source that can (sources sorted by name).
    pub fn interpolate(&self, clock: &QcClockId, t: Epoch) -> Option<(&QcIndexing, f64)> {
        let mut sources = self.sources.keys().collect::<Vec<_>>();
        sources.sort_by_key(|s| s.to_string());

        sources
            .into_iter()
            .find_map(|source| Some((source, self.interpolate_from(source, clock, t)?)))
    }

    /// Clock offset (s) of `clock` at `t`, linearly interpolated, from a specific source.
    pub fn interpolate_from(
        &self,
        source: &QcIndexing,
        clock: &QcClockId,
        t: Epoch,
    ) -> Option<f64> {
        let fifo = self.sources.get(source)?.get(clock)?;
        interpolate_linear(fifo, t, |c| (c.epoch, c.offset_s))
    }

    /// Onboard clock offset (s) of `sv` at `t`, from the first source that can.
    pub fn satellite_offset_s(&self, sv: SV, t: Epoch) -> Option<(&QcIndexing, f64)> {
        self.interpolate(&QcClockId::Satellite(sv), t)
    }

    /// Clock offset (s) of the ground station `name` at `t`, from the first source that can.
    pub fn station_offset_s(&self, name: &str, t: Epoch) -> Option<(&QcIndexing, f64)> {
        self.interpolate(&QcClockId::Station(name.to_string()), t)
    }

    /// Buffered states of `clock` from `source`, time ordered.
    pub fn samples(
        &self,
        source: &QcIndexing,
        clock: &QcClockId,
    ) -> Option<&VecDeque<QcPreciseClock>> {
        self.sources.get(source)?.get(clock)
    }

    /// Clocks described by this source.
    pub fn clocks_of(&self, source: &QcIndexing) -> Vec<QcClockId> {
        let mut clocks = self
            .sources
            .get(source)
            .map(|clocks| clocks.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        clocks.sort();
        clocks
    }

    /// Precise clocks sources that delivered something.
    pub fn sources(&self) -> impl Iterator<Item = &QcIndexing> {
        self.sources.keys()
    }

    /// Drops all expired states.
    pub fn prune(&mut self, t: Epoch) {
        self.sources.retain(|_, clocks| {
            clocks.retain(|_, fifo| {
                while fifo
                    .front()
                    .is_some_and(|c| t - c.epoch > PRECISE_CLOCKS_RETENTION)
                {
                    fifo.pop_front();
                }
                !fifo.is_empty()
            });
            !clocks.is_empty()
        });
    }

    /// Total number of buffered states.
    pub fn len(&self) -> usize {
        self.sources
            .values()
            .flat_map(|clocks| clocks.values())
            .map(|fifo| fifo.len())
            .sum()
    }

    /// Returns true if nothing is buffered.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Meteo observations, buffered per source and per observable.
#[derive(Debug, Default, Clone)]
pub struct QcMeteoStore {
    sources: HashMap<QcIndexing, HashMap<Observable, VecDeque<(Epoch, f64)>>>,
}

impl QcMeteoStore {
    /// Stores a new meteo observation delivered by `source`.
    pub fn push(&mut self, source: &QcIndexing, epoch: Epoch, sample: QcMeteoSample) {
        let fifo = self
            .sources
            .entry(source.clone())
            .or_default()
            .entry(sample.observable)
            .or_default();

        match fifo.back() {
            Some((last, _)) if *last > epoch => {
                let pos = fifo.partition_point(|(t, _)| *t <= epoch);
                fifo.insert(pos, (epoch, sample.value));
            }
            _ => fifo.push_back((epoch, sample.value)),
        }
    }

    /// Latest observation of `observable` by `source`, at or before `t`.
    pub fn latest(
        &self,
        source: &QcIndexing,
        observable: &Observable,
        t: Epoch,
    ) -> Option<(Epoch, f64)> {
        let fifo = self.sources.get(source)?.get(observable)?;
        let pos = fifo.partition_point(|(epoch, _)| *epoch <= t);
        (pos > 0).then(|| fifo[pos - 1])
    }

    /// Observation of `observable` by `source` at `t`, linearly interpolated.
    pub fn interpolate_from(
        &self,
        source: &QcIndexing,
        observable: &Observable,
        t: Epoch,
    ) -> Option<f64> {
        let fifo = self.sources.get(source)?.get(observable)?;
        interpolate_linear(fifo, t, |s| *s)
    }

    /// Observation of `observable` at `t`, linearly interpolated,
    /// from the first source that can (sources sorted by name).
    pub fn interpolate(&self, observable: &Observable, t: Epoch) -> Option<(&QcIndexing, f64)> {
        let mut sources = self.sources.keys().collect::<Vec<_>>();
        sources.sort_by_key(|s| s.to_string());

        sources
            .into_iter()
            .find_map(|source| Some((source, self.interpolate_from(source, observable, t)?)))
    }

    /// Buffered observations of `observable` by `source`, time ordered.
    pub fn samples(
        &self,
        source: &QcIndexing,
        observable: &Observable,
    ) -> Option<&VecDeque<(Epoch, f64)>> {
        self.sources.get(source)?.get(observable)
    }

    /// Observables of this source.
    pub fn observables_of(&self, source: &QcIndexing) -> Vec<Observable> {
        let mut observables = self
            .sources
            .get(source)
            .map(|obs| obs.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        observables.sort();
        observables
    }

    /// Meteo sources that delivered something.
    pub fn sources(&self) -> impl Iterator<Item = &QcIndexing> {
        self.sources.keys()
    }

    /// Drops all expired observations.
    pub fn prune(&mut self, t: Epoch) {
        self.sources.retain(|_, observables| {
            observables.retain(|_, fifo| {
                while fifo
                    .front()
                    .is_some_and(|(epoch, _)| t - *epoch > METEO_RETENTION)
                {
                    fifo.pop_front();
                }
                !fifo.is_empty()
            });
            !observables.is_empty()
        });
    }

    /// Total number of buffered observations.
    pub fn len(&self) -> usize {
        self.sources
            .values()
            .flat_map(|obs| obs.values())
            .map(|fifo| fifo.len())
            .sum()
    }

    /// Returns true if nothing is buffered.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    fn epoch(s: &str) -> Epoch {
        Epoch::from_str(s).unwrap()
    }

    fn clock(sv: SV, epoch: Epoch, offset_s: f64) -> QcPreciseClock {
        QcPreciseClock {
            clock: QcClockId::Satellite(sv),
            profile: ClockProfileType::AS,
            epoch,
            offset_s,
            offset_dev_s: None,
            drift_s_s: None,
        }
    }

    #[test]
    fn linear_interpolation() {
        let t0 = epoch("2020-06-25T00:00:00 GPST");
        let dt = Duration::from_seconds(30.0);

        let mut series = (0..10)
            .map(|i| (t0 + dt * i, 1.0E-4 + 1.0E-9 * i as f64))
            .collect::<VecDeque<_>>();

        // exact samples
        assert_eq!(interpolate_linear(&series, t0, |s| *s), Some(1.0E-4));
        assert_eq!(
            interpolate_linear(&series, t0 + dt * 9, |s| *s),
            Some(1.0E-4 + 9.0E-9)
        );

        // linear
        let y = interpolate_linear(&series, t0 + Duration::from_seconds(45.0), |s| *s).unwrap();
        assert!((y - (1.0E-4 + 1.5E-9)).abs() < 1.0E-18);

        // no extrapolation
        assert!(interpolate_linear(&series, t0 - Duration::from_seconds(1.0), |s| *s).is_none());
        assert!(interpolate_linear(&series, t0 + dt * 10, |s| *s).is_none());

        // data gap: 120 s missing (4 samples)
        series.retain(|(t, _)| *t <= t0 + dt * 3 || *t >= t0 + dt * 8);
        assert!(interpolate_linear(&series, t0 + dt * 5, |s| *s).is_none());
        assert!(interpolate_linear(&series, t0 + Duration::from_seconds(75.0), |s| *s).is_some());
    }

    #[test]
    fn clock_store() {
        let g01 = SV::from_str("G01").unwrap();
        let t0 = epoch("2020-06-25T00:00:00 GPST");
        let dt = Duration::from_seconds(30.0);
        let grg = QcIndexing::Agency("GRG".to_string());

        let mut store = QcClockStore::default();

        // out of order on purpose
        for i in [0, 2, 1, 3] {
            store.push(&grg, clock(g01, t0 + dt * i, 1.0E-5 * i as f64));
        }

        // not analysis results
        let mut monitoring = clock(g01, t0, 1.0);
        monitoring.profile = ClockProfileType::MS;
        store.push(&grg, monitoring);

        assert_eq!(store.len(), 4);
        assert_eq!(store.clocks_of(&grg), vec![QcClockId::Satellite(g01)]);

        let (source, offset) = store
            .satellite_offset_s(g01, t0 + Duration::from_seconds(15.0))
            .unwrap();
        assert_eq!(source, &grg);
        assert!((offset - 0.5E-5).abs() < 1.0E-18);

        assert!(store
            .satellite_offset_s(SV::from_str("G02").unwrap(), t0)
            .is_none());
        assert!(store.station_offset_s("ESBC", t0).is_none());

        // retention: 1 hour
        store.prune(t0 + Duration::from_seconds(3600.0 + 45.0));
        assert_eq!(store.len(), 2);

        store.prune(t0 + Duration::from_hours(3.0));
        assert!(store.is_empty());
    }

    #[test]
    fn meteo_store() {
        let t0 = epoch("2023-09-11T00:00:00 GPST");
        let dt = Duration::from_seconds(300.0);
        let pots = QcIndexing::Custom("POTS".to_string());

        let mut store = QcMeteoStore::default();

        for i in 0..4 {
            store.push(
                &pots,
                t0 + dt * i,
                QcMeteoSample {
                    observable: Observable::Pressure,
                    value: 1000.0 + i as f64,
                },
            );
        }

        assert_eq!(store.observables_of(&pots), vec![Observable::Pressure]);

        let t = t0 + Duration::from_seconds(450.0);

        assert_eq!(
            store.latest(&pots, &Observable::Pressure, t),
            Some((t0 + dt, 1001.0))
        );

        let (_, p) = store.interpolate(&Observable::Pressure, t).unwrap();
        assert!((p - 1001.5).abs() < 1.0E-9);

        assert!(store.latest(&pots, &Observable::Temperature, t).is_none());
        assert!(store
            .latest(
                &pots,
                &Observable::Pressure,
                t0 - Duration::from_seconds(1.0)
            )
            .is_none());

        store.prune(t0 + Duration::from_hours(4.0));
        assert!(store.is_empty());
    }
}
