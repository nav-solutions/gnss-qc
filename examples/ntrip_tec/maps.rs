//! TEC maps: the TEC estimates of a network of receivers, gridded into maps
//! (single layer model), formatted as IONEX. Same algorithm as the `obs2tec` command of rinex-cli.
//!
//! GNSS-Qc streams one leveled TEC estimate per line of sight and epoch, at the
//! ionospheric pierce point ([QcOutput::TecEstimate]), then the exact leveling of every
//! phase arc once it ended ([QcOutput::TecArc]). This module collects them and builds
//! the maps:
//!
//! 1. Vertical TEC of every pierce point, with the exact (a posteriori) leveling of its arc:
//!    `VTEC = (STEC(phase) + leveling) / obliquity factor`.
//!    Differential code biases (DCB) are **not** corrected (see the documentation).
//! 2. One map every interval `Tk` (UTC, IONEX): pierce points of `[Tk − interval/2, Tk + interval/2[`.
//!    In the Sun-fixed frame (default, IONEX specifications), a pierce point observed at `t`
//!    is moved to the longitude `λ + 360° (t − Tk) / 1 day` of the map `Tk`: the ionosphere
//!    follows the Sun far more than the Earth.
//! 3. Every node of the grid is the inverse distance weighted (1/d²) mean of the pierce points
//!    within the influence radius. The RMS is their weighted dispersion. Nodes with too few
//!    pierce points are left empty (9999 in IONEX).
use std::collections::{BTreeMap, BTreeSet, HashMap};

use gnss_qc::prelude::{Constellation, Duration, Epoch, QcOutput, QcOutputItem, TimeScale, SV};

use ionex::prelude::Linspace;

/// Seconds per day: rotation of the Sun-fixed frame (360° per day)
const DAY_S: f64 = 86400.0;

/// Extent of the maps
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridExtent {
    /// Bounding box of the pierce points, snapped onto the grid spacing
    Network,
    /// The whole world (IGS like grid)
    World,
    /// Latitudes (min, max) and longitudes (min, max), degrees
    Bounds {
        latitude_deg: (f64, f64),
        longitude_deg: (f64, f64),
    },
}

impl std::str::FromStr for GridExtent {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        match text.to_lowercase().as_str() {
            "network" | "auto" => return Ok(Self::Network),
            "world" | "global" => return Ok(Self::World),
            _ => {}
        }

        let invalid = || {
            format!(
                "invalid grid \"{}\": network, world or LAT_MIN,LAT_MAX,LON_MIN,LON_MAX",
                text
            )
        };

        let values = text
            .split(',')
            .map(|v| v.trim().parse::<f64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| invalid())?;

        match values.as_slice() {
            [lat0, lat1, lon0, lon1]
                if lat0 <= lat1
                    && lon0 <= lon1
                    && *lat0 >= -90.0
                    && *lat1 <= 90.0
                    && *lon0 >= -180.0
                    && *lon1 <= 180.0 =>
            {
                Ok(Self::Bounds {
                    latitude_deg: (*lat0, *lat1),
                    longitude_deg: (*lon0, *lon1),
                })
            }
            _ => Err(invalid()),
        }
    }
}

/// Parses a point of interest: "NAME:LAT,LON" (degrees)
pub fn parse_point(text: &str) -> Result<(String, f64, f64), String> {
    let invalid = || format!("invalid point \"{}\": NAME:LATITUDE,LONGITUDE", text);
    let (name, coordinates) = text.split_once(':').ok_or_else(invalid)?;
    let (lat, lon) = coordinates.split_once(',').ok_or_else(invalid)?;
    let (lat, lon) = (
        lat.trim().parse::<f64>().map_err(|_| invalid())?,
        lon.trim().parse::<f64>().map_err(|_| invalid())?,
    );
    if name.trim().is_empty() || lat.abs() > 90.0 || lon.abs() > 180.0 {
        return Err(invalid());
    }
    Ok((name.trim().to_string(), lat, lon))
}

/// Gridding of the TEC estimates
#[derive(Debug, Clone)]
pub struct Obs2TecSetup {
    /// Extent of the maps
    pub extent: GridExtent,
    /// Latitude spacing (°)
    pub dlat_deg: f64,
    /// Longitude spacing (°)
    pub dlon_deg: f64,
    /// One map every interval
    pub interval: Duration,
    /// Influence radius of a pierce point (°, great circle)
    pub radius_deg: f64,
    /// Minimal number of pierce points of a node
    pub min_samples: usize,
    /// Sun-fixed frame (IONEX specifications)
    pub rotation: bool,
    /// Altitude of the single layer (km)
    pub shell_altitude_km: f64,
    /// Earth radius of the single layer model (km)
    pub earth_radius_km: f64,
    /// Elevation mask of the estimates (°)
    pub elevation_mask_deg: f64,
    /// Points of interest: (name, latitude, longitude)
    pub points: Vec<(String, f64, f64)>,
}

impl Default for Obs2TecSetup {
    fn default() -> Self {
        Self {
            extent: GridExtent::Network,
            dlat_deg: 2.5,
            dlon_deg: 5.0,
            interval: Duration::from_hours(1.0),
            radius_deg: 5.0,
            min_samples: 3,
            rotation: true,
            shell_altitude_km: 450.0,
            earth_radius_km: 6371.0,
            elevation_mask_deg: 15.0,
            points: Vec::new(),
        }
    }
}

/// Phase arc: (receiver, satellite, code 1, code j, arc number)
type ArcKey = (String, SV, String, String, u32);

/// Final statistics of a phase arc
#[derive(Debug, Clone, Copy)]
pub struct ArcSummary {
    /// Exact leveling (TECu)
    pub leveling_tecu: f64,
}

/// One TEC estimate at a pierce point
#[derive(Debug, Clone)]
pub struct Sample {
    pub epoch: Epoch,
    pub receiver: String,
    pub sv: SV,
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub obliquity_factor: f64,
    pub phase_slant_tecu: f64,
    /// Running (causal) leveling, until the arc ends
    pub running_leveling_tecu: f64,
    key: ArcKey,
}

/// Accumulates the TEC estimates and the phase arcs
#[derive(Debug, Default)]
pub struct TecCollector {
    pub samples: Vec<Sample>,
    pub arcs: HashMap<(String, SV, String, String, u32), ArcSummary>,
}

impl TecCollector {
    /// Consumes one result
    pub fn consume(&mut self, item: &QcOutputItem) {
        match &item.payload {
            QcOutput::TecEstimate(e) => {
                self.samples.push(Sample {
                    // IONEX maps: UTC
                    epoch: item.epoch.to_time_scale(TimeScale::UTC),
                    receiver: e.receiver.to_string(),
                    sv: e.sv,
                    latitude_deg: e.latitude_deg,
                    longitude_deg: e.longitude_deg,
                    obliquity_factor: e.obliquity_factor,
                    phase_slant_tecu: e.phase_slant_tecu,
                    running_leveling_tecu: e.leveling_tecu,
                    key: (
                        e.receiver.to_string(),
                        e.sv,
                        e.code_1.to_string(),
                        e.code_j.to_string(),
                        e.arc,
                    ),
                });
            }
            QcOutput::TecArc(arc) => {
                self.arcs.insert(
                    (
                        arc.receiver.to_string(),
                        arc.sv,
                        arc.code_1.to_string(),
                        arc.code_j.to_string(),
                        arc.arc,
                    ),
                    ArcSummary {
                        leveling_tecu: arc.leveling_tecu,
                    },
                );
            }
            _ => {}
        }
    }

    /// Phase arc of this sample, if it ended
    pub fn arc(&self, sample: &Sample) -> Option<&ArcSummary> {
        self.arcs.get(&sample.key)
    }

    /// Slant TEC of this sample (TECu): leveled phase, exact leveling of its arc
    /// (running leveling when the arc did not end). DCB not corrected.
    pub fn slant_tecu(&self, sample: &Sample) -> f64 {
        let leveling = self
            .arc(sample)
            .map(|arc| arc.leveling_tecu)
            .unwrap_or(sample.running_leveling_tecu);
        sample.phase_slant_tecu + leveling
    }

    /// Vertical TEC of this sample (TECu)
    pub fn vertical_tecu(&self, sample: &Sample) -> f64 {
        self.slant_tecu(sample) / sample.obliquity_factor
    }

    /// Receivers of the network
    pub fn receivers(&self) -> BTreeSet<String> {
        self.samples.iter().map(|s| s.receiver.clone()).collect()
    }

    /// Satellites
    pub fn satellites(&self) -> BTreeSet<SV> {
        self.samples.iter().map(|s| s.sv).collect()
    }

    /// Constellations
    pub fn constellations(&self) -> BTreeSet<Constellation> {
        self.samples.iter().map(|s| s.sv.constellation).collect()
    }

    /// Builds the TEC maps of the pierce points of `[start, end[`
    pub fn grid(&self, setup: &Obs2TecSetup, start: Epoch, end: Epoch) -> Option<TecMaps> {
        grid(self, setup, start, end)
    }

    /// Drops the estimates (and the phase arcs) older than `epoch`
    pub fn prune(&mut self, epoch: Epoch) {
        self.samples.retain(|s| s.epoch >= epoch);
        let keys = self
            .samples
            .iter()
            .map(|s| s.key.clone())
            .collect::<std::collections::HashSet<_>>();
        self.arcs.retain(|k, _| keys.contains(k));
    }
}

/// One node (or point) of a map
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Node {
    /// Vertical TEC (TECu)
    pub tecu: f64,
    /// Weighted dispersion of the pierce points (TECu)
    pub rms_tecu: f64,
    /// Number of pierce points
    pub samples: usize,
}

/// One TEC map
#[derive(Debug, Clone, Default)]
pub struct TecMap {
    /// (latitude index, longitude index) → node
    pub nodes: BTreeMap<(usize, usize), Node>,
    /// Points of interest
    pub points: BTreeMap<String, Node>,
    /// Pierce points of the map
    pub samples: usize,
    /// Receivers contributing to the map
    pub receivers: usize,
}

/// The TEC maps
#[derive(Debug, Clone)]
pub struct TecMaps {
    /// Latitudes, North to South
    pub latitude: Linspace,
    /// Longitudes, West to East
    pub longitude: Linspace,
    /// Altitude of the single layer (km)
    pub altitude_km: f64,
    /// Interval between two maps
    pub interval: Duration,
    /// The maps
    pub maps: BTreeMap<Epoch, TecMap>,
}

impl TecMaps {
    /// Latitude of the i-th row (°)
    pub fn latitude_deg(&self, i: usize) -> f64 {
        self.latitude.start + i as f64 * self.latitude.spacing
    }

    /// Longitude of the j-th column (°)
    pub fn longitude_deg(&self, j: usize) -> f64 {
        self.longitude.start + j as f64 * self.longitude.spacing
    }

    /// Number of grid nodes
    pub fn num_nodes(&self) -> usize {
        self.latitude.num_points() * self.longitude.num_points()
    }
}

/// Longitude in [-180°, 180°[
fn wrap_longitude(longitude_deg: f64) -> f64 {
    (longitude_deg + 180.0).rem_euclid(360.0) - 180.0
}

/// Great circle distance (°)
pub fn angular_distance_deg(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (lat1, lon1) = (a.0.to_radians(), a.1.to_radians());
    let (lat2, lon2) = (b.0.to_radians(), b.1.to_radians());
    let h = ((lat2 - lat1) / 2.0).sin().powi(2)
        + lat1.cos() * lat2.cos() * ((lon2 - lon1) / 2.0).sin().powi(2);
    (2.0 * h.sqrt().min(1.0).asin()).to_degrees()
}

/// Inverse distance weighted mean
#[derive(Debug, Clone, Copy, Default)]
struct Accumulator {
    w: f64,
    wv: f64,
    wv2: f64,
    n: usize,
}

impl Accumulator {
    fn push(&mut self, distance_deg: f64, radius_deg: f64, value: f64) {
        // a pierce point on the node does not take all the weight
        let d = distance_deg.max(radius_deg / 10.0);
        let w = 1.0 / d.powi(2);
        self.w += w;
        self.wv += w * value;
        self.wv2 += w * value * value;
        self.n += 1;
    }

    fn node(&self, min_samples: usize) -> Option<Node> {
        if self.n < min_samples.max(1) || self.w <= 0.0 {
            return None;
        }
        let tecu = self.wv / self.w;
        Some(Node {
            tecu,
            rms_tecu: (self.wv2 / self.w - tecu * tecu).max(0.0).sqrt(),
            samples: self.n,
        })
    }
}

/// Grid axis from `min` to `max`, snapped outward onto multiples of the spacing
fn snapped(min: f64, max: f64, spacing: f64, limit: f64) -> (f64, f64) {
    let lo = ((min / spacing).floor() * spacing).max(-limit);
    let hi = ((max / spacing).ceil() * spacing).min(limit);
    (lo, hi)
}

fn grid(
    collector: &TecCollector,
    setup: &Obs2TecSetup,
    start: Epoch,
    end: Epoch,
) -> Option<TecMaps> {
    let interval_s = setup.interval.to_seconds();

    let samples = collector
        .samples
        .iter()
        .filter(|s| s.epoch >= start && s.epoch < end)
        .collect::<Vec<_>>();

    if samples.is_empty() || interval_s <= 0.0 {
        return None;
    }

    let first = samples.iter().map(|s| s.epoch).min()?;
    let last = samples.iter().map(|s| s.epoch).max()?;

    let t0 = first.round(setup.interval);

    // map of every sample, its coordinates in the frame of that map, vertical TEC
    let located = samples
        .iter()
        .map(|s| {
            let k = ((s.epoch - t0).to_seconds() / interval_s).round().max(0.0) as usize;
            let dt = (s.epoch - (t0 + setup.interval * k as i64)).to_seconds();
            let longitude = if setup.rotation {
                wrap_longitude(s.longitude_deg + 360.0 * dt / DAY_S)
            } else {
                s.longitude_deg
            };
            (k, s, s.latitude_deg, longitude, collector.vertical_tecu(s))
        })
        .collect::<Vec<_>>();

    let num_maps =
        ((last.round(setup.interval) - t0).to_seconds() / interval_s).round() as usize + 1;

    let (dlat, dlon) = (setup.dlat_deg.abs(), setup.dlon_deg.abs());

    let (lat, lon) = match setup.extent {
        GridExtent::World => {
            let limit = (90.0 / dlat).floor() * dlat;
            let limit = if limit >= 90.0 { 90.0 - dlat } else { limit };
            ((-limit, limit), (-180.0, 180.0))
        }
        GridExtent::Bounds {
            latitude_deg,
            longitude_deg,
        } => (
            snapped(latitude_deg.0, latitude_deg.1, dlat, 90.0),
            snapped(longitude_deg.0, longitude_deg.1, dlon, 180.0),
        ),
        GridExtent::Network => {
            let (mut lat_min, mut lat_max) = (f64::MAX, f64::MIN);
            let (mut lon_min, mut lon_max) = (f64::MAX, f64::MIN);
            for (_, _, lat, lon, _) in located.iter() {
                lat_min = lat_min.min(*lat);
                lat_max = lat_max.max(*lat);
                lon_min = lon_min.min(*lon);
                lon_max = lon_max.max(*lon);
            }
            (
                snapped(lat_min, lat_max, dlat, 90.0),
                snapped(lon_min, lon_max, dlon, 180.0),
            )
        }
    };

    // IONEX: North to South, West to East
    let latitude = if lat.0 == lat.1 {
        Linspace::new(lat.1, lat.1, 0.0)
    } else {
        Linspace::new(lat.1, lat.0, -dlat)
    }
    .ok()?;

    let longitude = if lon.0 == lon.1 {
        Linspace::new(lon.0, lon.0, 0.0)
    } else {
        Linspace::new(lon.0, lon.1, dlon)
    }
    .ok()?;

    let mut maps = TecMaps {
        latitude,
        longitude,
        altitude_km: setup.shell_altitude_km,
        interval: setup.interval,
        maps: BTreeMap::new(),
    };

    let (num_lat, num_lon) = (latitude.num_points(), longitude.num_points());
    let radius = setup.radius_deg.max(1.0E-3);

    let mut accumulators = vec![vec![Accumulator::default(); num_lat * num_lon]; num_maps];
    let mut points = vec![vec![Accumulator::default(); setup.points.len()]; num_maps];
    let mut receivers = vec![BTreeSet::<&str>::new(); num_maps];
    let mut counts = vec![0_usize; num_maps];

    for (k, sample, lat, lon, vtec) in located.iter() {
        let k = (*k).min(num_maps - 1);
        counts[k] += 1;
        receivers[k].insert(sample.receiver.as_str());

        // grid rows within the radius
        for i in 0..num_lat {
            let node_lat = maps.latitude_deg(i);
            if (node_lat - lat).abs() > radius {
                continue;
            }
            for j in 0..num_lon {
                let node_lon = maps.longitude_deg(j);
                let d = angular_distance_deg((node_lat, node_lon), (*lat, *lon));
                if d <= radius {
                    accumulators[k][i * num_lon + j].push(d, radius, *vtec);
                }
            }
        }

        for (p, (_, point_lat, point_lon)) in setup.points.iter().enumerate() {
            let d = angular_distance_deg((*point_lat, *point_lon), (*lat, *lon));
            if d <= radius {
                points[k][p].push(d, radius, *vtec);
            }
        }
    }

    for k in 0..num_maps {
        let epoch = t0 + setup.interval * k as i64;
        let mut map = TecMap {
            samples: counts[k],
            receivers: receivers[k].len(),
            ..Default::default()
        };
        for i in 0..num_lat {
            for j in 0..num_lon {
                if let Some(node) = accumulators[k][i * num_lon + j].node(setup.min_samples) {
                    map.nodes.insert((i, j), node);
                }
            }
        }
        for (p, (name, _, _)) in setup.points.iter().enumerate() {
            if let Some(node) = points[k][p].node(setup.min_samples) {
                map.points.insert(name.clone(), node);
            }
        }
        maps.maps.insert(epoch, map);
    }

    Some(maps)
}

/// Formats these maps as IONEX
pub fn ionex(
    collector: &TecCollector,
    setup: &Obs2TecSetup,
    maps: &TecMaps,
    program: &str,
) -> Option<ionex::prelude::IONEX> {
    use ionex::prelude::{Header, Key, MappingFunction, Record, ReferenceSystem, IONEX, TEC};

    let mut record = Record::default();

    for (epoch, map) in maps.maps.iter() {
        for ((i, j), node) in map.nodes.iter() {
            record.insert(
                Key::from_decimal_degrees_km(
                    *epoch,
                    maps.latitude_deg(*i),
                    maps.longitude_deg(*j),
                    maps.altitude_km,
                ),
                TEC::from_tecu(node.tecu).with_rms(node.rms_tecu),
            );
        }
    }

    let first = *maps.maps.keys().next()?;
    let last = *maps.maps.keys().next_back()?;

    let constellations = collector.constellations();
    let constellation = if constellations.len() == 1 {
        *constellations.iter().next().unwrap()
    } else {
        Constellation::Mixed
    };

    let receivers = collector.receivers();

    let mut header = Header::default()
        .with_latitude_grid(maps.latitude)
        .with_longitude_grid(maps.longitude)
        .with_altitude_grid(Linspace::new(maps.altitude_km, maps.altitude_km, 0.0).ok()?)
        .with_mapping_function(MappingFunction::CosZ)
        .with_elevation_cutoff(setup.elevation_mask_deg as f32)
        .with_base_radius_km(setup.earth_radius_km as f32)
        .with_map_dimension(2)
        .with_reference_system(ReferenceSystem::Constellation(constellation))
        .with_description("Real-time vertical TEC from NTRIP streams (GNSS-Qc)")
        .with_epoch_of_first_map(first)
        .with_epoch_of_last_map(last)
        .with_number_of_maps(maps.maps.len() as u32);

    header.sampling_period = maps.interval;
    header.num_stations = receivers.len() as u32;
    header.num_satellites = collector.satellites().len() as u16;
    header.program = Some(program.to_string());
    header.run_by = None;

    for comment in [
        "TEC estimated in real-time from the NTRIP streams of a".to_string(),
        "network: dual frequency code and phase, phase leveled".to_string(),
        "onto the code over each continuous phase arc (running".to_string(),
        "leveling, exact leveling once the arc ended),".to_string(),
        format!(
            "single layer model ({} km), elevation mask {} deg.",
            setup.shell_altitude_km, setup.elevation_mask_deg
        ),
        "UNCALIBRATED: satellite and receiver differential code biases".to_string(),
        "(DCB) are NOT corrected. TEC biased by the DCB of each pair.".to_string(),
        format!(
            "Gridding: inverse distance weighted, radius {} deg, {}",
            setup.radius_deg,
            if setup.rotation {
                "Sun-fixed"
            } else {
                "Earth-fixed"
            }
        ),
        format!(
            "frame, >= {} pierce points per node. RMS: dispersion.",
            setup.min_samples
        ),
        format!(
            "Receivers: {}",
            receivers.iter().cloned().collect::<Vec<_>>().join(" ")
        ),
    ] {
        // IONEX comments: 60 characters
        let mut line = String::new();
        for word in comment.split_whitespace() {
            if !line.is_empty() && line.len() + word.len() + 1 > 60 {
                header.comments.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        if !line.is_empty() {
            header.comments.push(line);
        }
    }

    Some(IONEX::new(header, record))
}
#[cfg(test)]
mod test {
    use super::*;
    use gnss_qc::prelude::{QcIndexing, QcTecArc, QcTecEstimate};
    use std::str::FromStr;

    fn estimate(
        receiver: &str,
        sv: &str,
        arc: u32,
        (lat, lon): (f64, f64),
        phase: f64,
        running: f64,
    ) -> QcOutput {
        QcOutput::TecEstimate(Box::new(QcTecEstimate {
            receiver: QcIndexing::Receiver(receiver.to_string()),
            sv: SV::from_str(sv).unwrap(),
            code_1: "C1W".parse().unwrap(),
            code_j: "C2W".parse().unwrap(),
            phase_1: "L1W".parse().unwrap(),
            phase_j: "L2W".parse().unwrap(),
            arc,
            arc_samples: 1,
            elevation_deg: 90.0,
            azimuth_deg: 0.0,
            latitude_deg: lat,
            longitude_deg: lon,
            altitude_km: 450.0,
            obliquity_factor: 1.0,
            code_slant_tecu: phase + running,
            phase_slant_tecu: phase,
            leveling_tecu: running,
            slant_tecu: phase + running,
            vertical_tecu: phase + running,
            map_vertical_tecu: None,
        }))
    }

    fn arc(receiver: &str, sv: &str, arc: u32, leveling: f64, t: Epoch) -> QcOutput {
        QcOutput::TecArc(Box::new(QcTecArc {
            receiver: QcIndexing::Receiver(receiver.to_string()),
            sv: SV::from_str(sv).unwrap(),
            code_1: "C1W".parse().unwrap(),
            code_j: "C2W".parse().unwrap(),
            phase_1: "L1W".parse().unwrap(),
            phase_j: "L2W".parse().unwrap(),
            arc,
            start: t,
            end: t,
            samples: 10,
            leveling_tecu: leveling,
            code_noise_tecu: 1.0,
        }))
    }

    /// Maps of every estimate
    fn all(collector: &TecCollector, setup: &Obs2TecSetup) -> Option<TecMaps> {
        collector.grid(
            setup,
            Epoch::from_str("2000-01-01T00:00:00 UTC").unwrap(),
            Epoch::from_str("2100-01-01T00:00:00 UTC").unwrap(),
        )
    }

    fn item(epoch: Epoch, payload: QcOutput) -> QcOutputItem {
        QcOutputItem { epoch, payload }
    }

    #[test]
    fn grid_extents() {
        assert_eq!(
            GridExtent::from_str("network").unwrap(),
            GridExtent::Network
        );
        assert_eq!(GridExtent::from_str("World").unwrap(), GridExtent::World);
        assert_eq!(
            GridExtent::from_str("35, 70, -10, 30").unwrap(),
            GridExtent::Bounds {
                latitude_deg: (35.0, 70.0),
                longitude_deg: (-10.0, 30.0)
            }
        );
        assert!(GridExtent::from_str("70,35,-10,30").is_err());
        assert!(GridExtent::from_str("35,70,-10").is_err());

        assert_eq!(
            parse_point("Esbjerg:55.5,8.4").unwrap(),
            ("Esbjerg".to_string(), 55.5, 8.4)
        );
        assert!(parse_point("55.5,8.4").is_err());
        assert!(parse_point("A:95,8.4").is_err());
    }

    #[test]
    fn distances() {
        assert!(angular_distance_deg((10.0, 20.0), (10.0, 20.0)).abs() < 1.0E-9);
        assert!((angular_distance_deg((0.0, 0.0), (1.0, 0.0)) - 1.0).abs() < 1.0E-9);
        // across the antimeridian
        assert!((angular_distance_deg((0.0, 179.5), (0.0, -179.5)) - 1.0).abs() < 1.0E-9);
        assert_eq!(wrap_longitude(190.0), -170.0);
        assert_eq!(wrap_longitude(-180.0), -180.0);
    }

    /// Leveling a posteriori, gridding, Sun-fixed rotation
    #[test]
    fn gridding() {
        let t0 = Epoch::from_str("2020-06-25T12:00:00 UTC").unwrap();
        let mut collector = TecCollector::default();

        // a uniform 10 TECu ionosphere seen around (55°N, 10°E), map 12:00:
        // running leveling still 2 TECu off, exact leveling given by the arc
        for (n, (lat, lon)) in [(55.0, 10.0), (56.0, 10.0), (55.0, 11.0), (54.0, 9.0)]
            .iter()
            .enumerate()
        {
            let sv = format!("G{:02}", n + 1);
            collector.consume(&item(t0, estimate("ESBC", &sv, 0, (*lat, *lon), 4.0, 8.0)));
            collector.consume(&item(t0, arc("ESBC", &sv, 0, 6.0, t0)));
        }

        // 13:15 (map 13:00, 15 minutes later): 3.75° further East in the Sun-fixed frame,
        // the arc did not end (running leveling)
        let t1 = t0 + Duration::from_seconds(4500.0);
        for sv in ["G10", "G11", "G12"] {
            collector.consume(&item(t1, estimate("MOJN", sv, 0, (55.0, 10.0), 1.0, 19.0)));
        }

        assert_eq!(collector.receivers().len(), 2);
        assert_eq!(collector.satellites().len(), 7);
        assert!((collector.vertical_tecu(&collector.samples[0]) - 10.0).abs() < 1.0E-9);
        assert!((collector.vertical_tecu(&collector.samples[4]) - 20.0).abs() < 1.0E-9);

        let setup = Obs2TecSetup {
            dlat_deg: 1.0,
            dlon_deg: 1.0,
            radius_deg: 1.5,
            min_samples: 1,
            points: vec![("Esbjerg".to_string(), 55.5, 8.5)],
            ..Default::default()
        };

        let maps = all(&collector, &setup).unwrap();

        assert_eq!(maps.maps.len(), 2, "two maps: 12:00 and 13:00");
        assert_eq!(maps.maps.keys().next().unwrap().time_scale, TimeScale::UTC);
        assert_eq!(maps.latitude.start, 56.0, "North to South");
        assert_eq!(maps.latitude.end, 54.0);
        assert_eq!(maps.longitude.start, 9.0);
        assert_eq!(
            maps.longitude.end, 14.0,
            "rotated pierce points of 13:15 at 13.75°E"
        );

        let first = maps.maps.get(&t0).unwrap();
        assert_eq!(first.samples, 4);
        assert_eq!(first.receivers, 1);
        for node in first.nodes.values() {
            assert!((node.tecu - 10.0).abs() < 1.0E-9);
            assert!(node.rms_tecu < 1.0E-6);
        }
        // (55°N, 10°E)
        assert!(first.nodes.contains_key(&(1, 1)));
        // far from any pierce point
        assert!(!first.nodes.contains_key(&(0, 5)));
        assert!((first.points["Esbjerg"].tecu - 10.0).abs() < 1.0E-9);

        let second = maps.maps.get(&(t0 + Duration::from_hours(1.0))).unwrap();
        assert_eq!(second.samples, 3);
        // (55°N, 14°E): next to the rotated pierce points (13.75°E)
        let node = second.nodes.get(&(1, 5)).unwrap();
        assert!((node.tecu - 20.0).abs() < 1.0E-9);
        assert_eq!(node.samples, 3);
        // nothing left at 10°E
        assert!(!second.nodes.contains_key(&(1, 1)));

        // Earth-fixed maps
        let maps = all(
            &collector,
            &Obs2TecSetup {
                rotation: false,
                ..setup.clone()
            },
        )
        .unwrap();
        assert_eq!(maps.longitude.end, 11.0);

        // minimal number of pierce points per node
        let maps = all(
            &collector,
            &Obs2TecSetup {
                min_samples: 4,
                ..setup
            },
        )
        .unwrap();
        assert!(maps.maps.values().nth(1).unwrap().nodes.is_empty());
    }

    /// IONEX maps: round UTC epochs, whatever the timescale of the observations
    #[test]
    fn utc_maps() {
        let t = Epoch::from_str("2020-06-25T12:00:00 GPST").unwrap();
        let mut collector = TecCollector::default();
        collector.consume(&item(t, estimate("A", "G01", 0, (55.0, 10.0), 5.0, 5.0)));

        let maps = all(
            &collector,
            &Obs2TecSetup {
                min_samples: 1,
                ..Default::default()
            },
        )
        .unwrap();

        let first = *maps.maps.keys().next().unwrap();
        assert_eq!(first, Epoch::from_str("2020-06-25T12:00:00 UTC").unwrap());
        assert_eq!(first.to_string(), "2020-06-25T12:00:00 UTC");
    }

    #[test]
    fn world_grid() {
        let t0 = Epoch::from_str("2020-06-25T00:00:00 GPST").unwrap();
        let mut collector = TecCollector::default();
        collector.consume(&item(t0, estimate("A", "G01", 0, (0.0, 179.0), 5.0, 5.0)));

        let maps = all(
            &collector,
            &Obs2TecSetup {
                extent: GridExtent::World,
                rotation: false,
                min_samples: 1,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(maps.latitude.start, 87.5);
        assert_eq!(maps.latitude.end, -87.5);
        assert_eq!(maps.latitude.num_points(), 71);
        assert_eq!(maps.longitude.num_points(), 73);

        let map = maps.maps.values().next().unwrap();
        // across the antimeridian: 180°E and 180°W
        let equator = 35;
        assert_eq!(maps.latitude_deg(equator), 0.0);
        assert!(map.nodes.contains_key(&(equator, 72)));
        assert!(map.nodes.contains_key(&(equator, 0)));
    }
}

#[cfg(test)]
mod windows {
    use super::*;
    use gnss_qc::prelude::{QcIndexing, QcTecEstimate};
    use std::str::FromStr;

    fn sample(t: Epoch, sv: &str, vtec: f64) -> QcOutputItem {
        QcOutputItem {
            epoch: t,
            payload: QcOutput::TecEstimate(Box::new(QcTecEstimate {
                receiver: QcIndexing::Custom("ESBC00DNK0".to_string()),
                sv: SV::from_str(sv).unwrap(),
                code_1: "C1W".parse().unwrap(),
                code_j: "C2W".parse().unwrap(),
                phase_1: "L1C".parse().unwrap(),
                phase_j: "L2W".parse().unwrap(),
                arc: 0,
                arc_samples: 1,
                elevation_deg: 90.0,
                azimuth_deg: 0.0,
                latitude_deg: 55.0,
                longitude_deg: 10.0,
                altitude_km: 450.0,
                obliquity_factor: 1.0,
                code_slant_tecu: vtec,
                phase_slant_tecu: vtec,
                leveling_tecu: 0.0,
                slant_tecu: vtec,
                vertical_tecu: vtec,
                map_vertical_tecu: None,
            })),
        }
    }

    /// Real-time: maps of a time window (one day), pruning, IONEX round trip
    #[test]
    fn daily_window_and_ionex() {
        let t0 = Epoch::from_str("2020-06-25T23:00:00 UTC").unwrap();
        let mut collector = TecCollector::default();

        // 23:00 to 01:00 (next day), every 5 minutes, 3 satellites
        for k in 0..24 {
            let t = t0 + Duration::from_seconds(300.0 * k as f64);
            for sv in ["G01", "G02", "G03"] {
                collector.consume(&sample(t, sv, 10.0 + k as f64 / 10.0));
            }
        }

        let setup = Obs2TecSetup {
            interval: Duration::from_hours(1.0),
            min_samples: 3,
            rotation: false,
            ..Default::default()
        };

        // the day 2020-06-25 ends at the map 2020-06-26 00:00 (± 30 min)
        let day = Epoch::from_str("2020-06-25T00:00:00 UTC").unwrap();
        let maps = collector
            .grid(
                &setup,
                day - Duration::from_seconds(60.0 * 30.0),
                day + Duration::from_hours(24.5),
            )
            .unwrap();

        let epochs = maps.maps.keys().map(|t| t.to_string()).collect::<Vec<_>>();
        assert_eq!(
            epochs,
            ["2020-06-25T23:00:00 UTC", "2020-06-26T00:00:00 UTC"]
        );

        let ionex = ionex(&collector, &setup, &maps, "test").unwrap();
        assert_eq!(ionex.header.number_of_maps, 2);
        assert!(ionex.header.comments.iter().all(|c| c.len() <= 60));
        assert!(ionex
            .header
            .comments
            .iter()
            .any(|c| c.contains("UNCALIBRATED")));

        let path = std::env::temp_dir().join("gnss-qc-ntrip-tec-test.INX");
        ionex.to_file(&path).unwrap();
        let parsed = ionex::prelude::IONEX::from_file(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(parsed.header.number_of_maps, 2);
        assert_eq!(parsed.record.epochs_iter().count(), 2);

        // pruning: the estimates of the previous day are released
        collector.prune(Epoch::from_str("2020-06-26T00:00:00 UTC").unwrap());
        assert_eq!(collector.samples.len(), 12 * 3);
    }
}
