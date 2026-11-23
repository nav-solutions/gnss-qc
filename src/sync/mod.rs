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

use crate::io::{input::indexing::QcIndexing, source::QcEphemeris};

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

    /// Iterates over all buffered data points (all satellites).
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.inner.values().flat_map(|fifo| fifo.iter())
    }

    /// Satellites for which something is buffered.
    pub fn satellites(&self) -> impl Iterator<Item = SV> + '_ {
        self.inner.keys().copied()
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
}
