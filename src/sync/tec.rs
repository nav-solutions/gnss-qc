//! TEC maps buffers and interpolation.
use std::collections::{BTreeMap, BTreeSet, HashMap};

use hifitime::Epoch;

use crate::io::{input::indexing::QcIndexing, source::QcTecPoint};

/// Coordinates are stored with a 1E-3 resolution (°, km)
const RESOLUTION: f64 = 1.0E3;

fn quantize(value: f64) -> i64 {
    (value * RESOLUTION).round() as i64
}

fn real(value: i64) -> f64 {
    value as f64 / RESOLUTION
}

/// Temporal interpolation strategy of the TEC maps, when an instant does not line up
/// with a map. These are the strategies of the IONEX specifications (Schaer et al., 1998).
/// When the instant lines up with a map, all strategies return that map.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QcTecInterpolation {
    /// Nearest map in time (discontinuities).
    NearestMap,

    /// Linear interpolation between the two consecutive maps, at the same
    /// (Earth fixed) coordinates.
    Linear,

    /// Linear interpolation between the two consecutive maps, each rotated around the
    /// Earth axis by the time difference (360° per day), to follow the Sun
    /// (the ionosphere is strongly correlated with the Sun position):
    ///
    /// E(t, φ, λ) = (T1 - t)/(T1 - T0) E0(φ, λ + (t - T0)) + (t - T0)/(T1 - T0) E1(φ, λ + (t - T1))
    ///
    /// Recommended by the IONEX specifications. Default.
    #[default]
    RotatedMaps,
}

impl QcTecInterpolation {
    /// Earth rotation rate relative to the Sun (°/s): 360° per day
    pub const SUN_FIXED_ROTATION_DEG_S: f64 = 360.0 / 86400.0;
}

/// Interpolated TEC value
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QcTecValue {
    /// Vertical TEC (TECu)
    pub tecu: f64,

    /// Possible RMS of the TEC (TECu)
    pub rms_tecu: Option<f64>,
}

impl QcTecValue {
    fn lerp(&self, rhs: &Self, w: f64) -> Self {
        Self {
            tecu: self.tecu + (rhs.tecu - self.tecu) * w,
            rms_tecu: match (self.rms_tecu, rhs.rms_tecu) {
                (Some(a), Some(b)) => Some(a + (b - a) * w),
                _ => None,
            },
        }
    }
}

/// One TEC map (one epoch): grid nodes, per altitude layer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QcTecMap {
    /// (altitude, latitude, longitude) → value
    nodes: BTreeMap<(i64, i64, i64), QcTecValue>,
    latitudes: BTreeSet<i64>,
    longitudes: BTreeSet<i64>,
    altitudes: BTreeSet<i64>,
}

impl QcTecMap {
    /// Inserts a new node
    pub fn insert(&mut self, point: &QcTecPoint) {
        let (alt, lat, lon) = (
            quantize(point.altitude_km),
            quantize(point.latitude_deg),
            quantize(point.longitude_deg),
        );

        self.altitudes.insert(alt);
        self.latitudes.insert(lat);
        self.longitudes.insert(lon);

        self.nodes.insert(
            (alt, lat, lon),
            QcTecValue {
                tecu: point.tecu,
                rms_tecu: point.rms_tecu,
            },
        );
    }

    /// Number of nodes
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns true if this map has no node
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Iterates over the nodes, sorted by altitude, latitude, longitude
    pub fn iter(&self) -> impl Iterator<Item = QcTecPoint> + '_ {
        self.nodes
            .iter()
            .map(|((alt, lat, lon), value)| QcTecPoint {
                latitude_deg: real(*lat),
                longitude_deg: real(*lon),
                altitude_km: real(*alt),
                tecu: value.tecu,
                rms_tecu: value.rms_tecu,
            })
    }

    /// Altitude layers (km)
    pub fn altitudes_km(&self) -> Vec<f64> {
        self.altitudes.iter().map(|alt| real(*alt)).collect()
    }

    /// Value of the node at these coordinates, if it exists
    pub fn node(
        &self,
        latitude_deg: f64,
        longitude_deg: f64,
        altitude_km: f64,
    ) -> Option<QcTecValue> {
        self.nodes
            .get(&(
                quantize(altitude_km),
                quantize(latitude_deg),
                quantize(longitude_deg),
            ))
            .copied()
    }

    /// Returns true if this map covers 360° of longitude
    pub fn is_worldwide(&self) -> bool {
        match (self.longitudes.first(), self.longitudes.last()) {
            (Some(min), Some(max)) => max - min >= quantize(360.0) - 1,
            _ => false,
        }
    }

    /// Wraps the longitude into the longitude range of a worldwide map
    fn wrap(&self, longitude: f64) -> f64 {
        match (self.is_worldwide(), self.longitudes.first()) {
            (true, Some(min)) => {
                let min = real(*min);
                min + (longitude - min).rem_euclid(360.0)
            }
            _ => longitude,
        }
    }

    /// Surrounding grid values of `value` in `axis`: (lower, upper)
    fn bracket(axis: &BTreeSet<i64>, value: i64) -> Option<(i64, i64)> {
        let lower = axis.range(..=value).next_back()?;
        let upper = axis.range(value..).next()?;
        Some((*lower, *upper))
    }

    /// Spatial interpolation: bilinear interpolation of the 4 surrounding nodes
    /// (IONEX specifications), nodes are returned as is. The altitude must be one of
    /// the layers of the map; `None` selects the single layer of 2D maps.
    pub fn interpolate(
        &self,
        latitude_deg: f64,
        longitude_deg: f64,
        altitude_km: Option<f64>,
    ) -> Option<QcTecValue> {
        let alt = match altitude_km {
            Some(alt) => quantize(alt),
            None if self.altitudes.len() == 1 => *self.altitudes.first()?,
            None => return None,
        };

        let lon = self.wrap(longitude_deg);
        let (lat, lon_q) = (quantize(latitude_deg), quantize(lon));

        let (lat0, lat1) = Self::bracket(&self.latitudes, lat)?;
        let (lon0, lon1) = Self::bracket(&self.longitudes, lon_q)?;

        let node = |lat: i64, lon: i64| self.nodes.get(&(alt, lat, lon)).copied();

        let p = if lat1 == lat0 {
            0.0
        } else {
            (latitude_deg - real(lat0)) / (real(lat1) - real(lat0))
        };

        let q = if lon1 == lon0 {
            0.0
        } else {
            (lon - real(lon0)) / (real(lon1) - real(lon0))
        };

        let (e00, e10, e01, e11) = (
            node(lat0, lon0)?,
            node(lat0, lon1)?,
            node(lat1, lon0)?,
            node(lat1, lon1)?,
        );

        // E = (1-p)(1-q) E00 + (1-p) q E10 + p (1-q) E01 + p q E11
        let bilinear = |a: f64, b: f64, c: f64, d: f64| {
            (1.0 - p) * (1.0 - q) * a + (1.0 - p) * q * b + p * (1.0 - q) * c + p * q * d
        };

        Some(QcTecValue {
            tecu: bilinear(e00.tecu, e10.tecu, e01.tecu, e11.tecu),
            rms_tecu: match (e00.rms_tecu, e10.rms_tecu, e01.rms_tecu, e11.rms_tecu) {
                (Some(a), Some(b), Some(c), Some(d)) => Some(bilinear(a, b, c, d)),
                _ => None,
            },
        })
    }
}

/// Two maps surrounding an instant: ((T0, map), (T1, map))
pub type QcTecMapBracket<'a> = ((Epoch, &'a QcTecMap), (Epoch, &'a QcTecMap));

/// TEC maps, buffered per source (agency) and per epoch.
/// The two latest maps that precede the epoch being processed are kept,
/// with every map ahead of it (lookahead).
#[derive(Debug, Default, Clone)]
pub struct QcTecStore {
    sources: HashMap<QcIndexing, BTreeMap<Epoch, QcTecMap>>,
}

impl QcTecStore {
    /// Stores a new TEC map node delivered by `source`.
    pub fn push(&mut self, source: &QcIndexing, epoch: Epoch, point: QcTecPoint) {
        self.sources
            .entry(source.clone())
            .or_default()
            .entry(epoch)
            .or_default()
            .insert(&point);
    }

    /// TEC maps sources that delivered something.
    pub fn sources(&self) -> impl Iterator<Item = &QcIndexing> {
        self.sources.keys()
    }

    /// Buffered maps of this source, in chronological order
    pub fn maps(&self, source: &QcIndexing) -> Option<&BTreeMap<Epoch, QcTecMap>> {
        self.sources.get(source)
    }

    /// Map of this source at this epoch, if it exists
    pub fn map(&self, source: &QcIndexing, epoch: Epoch) -> Option<&QcTecMap> {
        self.sources.get(source)?.get(&epoch)
    }

    /// Maps surrounding `t`: ((T0, map), (T1, map)), T0 = T1 when `t` lines up with a map
    pub fn bracket(&self, source: &QcIndexing, t: Epoch) -> Option<QcTecMapBracket<'_>> {
        let maps = self.sources.get(source)?;
        let (t0, m0) = maps.range(..=t).next_back()?;
        let (t1, m1) = maps.range(t..).next()?;
        Some(((*t0, m0), (*t1, m1)))
    }

    /// Vertical TEC of `source` at `t` and these coordinates: temporal interpolation
    /// (see [QcTecInterpolation]) of the spatial (bilinear) interpolations.
    /// No extrapolation: `t` must lie between two maps.
    pub fn tec_from(
        &self,
        source: &QcIndexing,
        t: Epoch,
        latitude_deg: f64,
        longitude_deg: f64,
        altitude_km: Option<f64>,
        method: QcTecInterpolation,
    ) -> Option<QcTecValue> {
        let ((t0, m0), (t1, m1)) = self.bracket(source, t)?;

        if t0 == t1 {
            return m0.interpolate(latitude_deg, longitude_deg, altitude_km);
        }

        let w = (t - t0).to_seconds() / (t1 - t0).to_seconds();

        match method {
            QcTecInterpolation::NearestMap => {
                let map = if w <= 0.5 { m0 } else { m1 };
                map.interpolate(latitude_deg, longitude_deg, altitude_km)
            }
            QcTecInterpolation::Linear => {
                let e0 = m0.interpolate(latitude_deg, longitude_deg, altitude_km)?;
                let e1 = m1.interpolate(latitude_deg, longitude_deg, altitude_km)?;
                Some(e0.lerp(&e1, w))
            }
            QcTecInterpolation::RotatedMaps => {
                let shift = |tm: Epoch| {
                    longitude_deg
                        + (t - tm).to_seconds() * QcTecInterpolation::SUN_FIXED_ROTATION_DEG_S
                };
                let e0 = m0.interpolate(latitude_deg, shift(t0), altitude_km)?;
                let e1 = m1.interpolate(latitude_deg, shift(t1), altitude_km)?;
                Some(e0.lerp(&e1, w))
            }
        }
    }

    /// Vertical TEC at `t` and these coordinates, from the first source that can
    /// (sources sorted by name), see [Self::tec_from].
    pub fn tec(
        &self,
        t: Epoch,
        latitude_deg: f64,
        longitude_deg: f64,
        method: QcTecInterpolation,
    ) -> Option<(&QcIndexing, QcTecValue)> {
        let mut sources = self.sources.keys().collect::<Vec<_>>();
        sources.sort_by_key(|s| s.to_string());

        sources.into_iter().find_map(|source| {
            Some((
                source,
                self.tec_from(source, t, latitude_deg, longitude_deg, None, method)?,
            ))
        })
    }

    /// Drops the maps that can no longer be used at `t`: every map that precedes
    /// the two latest maps at or before `t`.
    pub fn prune(&mut self, t: Epoch) {
        for maps in self.sources.values_mut() {
            let past = maps.range(..=t).map(|(e, _)| *e).collect::<Vec<_>>();
            if past.len() > 2 {
                let keep_from = past[past.len() - 2];
                maps.retain(|e, _| *e >= keep_from);
            }
        }
    }

    /// Total number of buffered nodes
    pub fn len(&self) -> usize {
        self.sources
            .values()
            .flat_map(|maps| maps.values())
            .map(|map| map.len())
            .sum()
    }

    /// Returns true if nothing is buffered
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use hifitime::Duration;
    use std::str::FromStr;

    /// Worldwide 2.5° x 5° map, TEC = a + b.lat + c.lon (linear: bilinear is exact)
    fn map(a: f64) -> QcTecMap {
        let mut map = QcTecMap::default();
        let mut lat = -87.5;
        while lat <= 87.5 {
            let mut lon = -180.0;
            while lon <= 180.0 {
                map.insert(&QcTecPoint {
                    latitude_deg: lat,
                    longitude_deg: lon,
                    altitude_km: 450.0,
                    tecu: a + 0.1 * lat + 0.01 * lon,
                    rms_tecu: Some(1.0),
                });
                lon += 5.0;
            }
            lat += 2.5;
        }
        map
    }

    #[test]
    fn spatial_interpolation() {
        let map = map(10.0);
        assert!(map.is_worldwide());
        assert_eq!(map.len(), 71 * 73);

        // node
        let v = map.interpolate(42.5, 0.0, None).unwrap();
        assert!((v.tecu - 14.25).abs() < 1.0E-12);

        // bilinear (exact for a linear field)
        let v = map.interpolate(43.6, 1.44, None).unwrap();
        assert!((v.tecu - (10.0 + 4.36 + 0.0144)).abs() < 1.0E-9);
        assert!((v.rms_tecu.unwrap() - 1.0).abs() < 1.0E-12);

        // longitude wrapping: 190° = -170°
        let a = map.interpolate(10.0, 190.0, None).unwrap();
        let b = map.interpolate(10.0, -170.0, None).unwrap();
        assert!((a.tecu - b.tecu).abs() < 1.0E-12);

        // outside of the latitude range, unknown layer
        assert!(map.interpolate(89.0, 0.0, None).is_none());
        assert!(map.interpolate(10.0, 0.0, Some(350.0)).is_none());
    }

    #[test]
    fn temporal_interpolation() {
        let t0 = Epoch::from_str("2021-01-09T00:00:00 UTC").unwrap();
        let dt = Duration::from_hours(1.0);
        let src = QcIndexing::Agency("AIUB".to_string());

        let mut store = QcTecStore::default();
        for (i, a) in [10.0, 20.0, 30.0].iter().enumerate() {
            for point in map(*a).iter() {
                store.push(&src, t0 + dt * i as i64, point);
            }
        }

        let t = t0 + Duration::from_seconds(1800.0);

        let linear = store
            .tec_from(&src, t, 42.5, 0.0, None, QcTecInterpolation::Linear)
            .unwrap();
        assert!((linear.tecu - (15.0 + 4.25)).abs() < 1.0E-9);

        let nearest = store
            .tec_from(&src, t, 42.5, 0.0, None, QcTecInterpolation::NearestMap)
            .unwrap();
        assert!((nearest.tecu - (10.0 + 4.25)).abs() < 1.0E-9);

        // rotated: maps shifted by +7.5° (T0) and -7.5° (T1): the longitude terms cancel
        let rotated = store
            .tec_from(&src, t, 42.5, 0.0, None, QcTecInterpolation::RotatedMaps)
            .unwrap();
        assert!((rotated.tecu - (15.0 + 4.25)).abs() < 1.0E-9);

        let rotated = store
            .tec_from(&src, t, 42.5, 10.0, None, QcTecInterpolation::RotatedMaps)
            .unwrap();
        let expected = 0.5 * (10.0 + 4.25 + 0.01 * 17.5) + 0.5 * (20.0 + 4.25 + 0.01 * 2.5);
        assert!((rotated.tecu - expected).abs() < 1.0E-9);

        // no extrapolation
        assert!(store
            .tec(t0 - dt, 42.5, 0.0, QcTecInterpolation::Linear)
            .is_none());

        // the two latest maps preceding t are kept
        store.prune(t0 + dt * 2);
        assert_eq!(store.maps(&src).unwrap().len(), 2);
        assert!(store.map(&src, t0).is_none());
    }
}
