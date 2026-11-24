//! TEC maps analysis and temporal interpolation.
use std::collections::HashMap;

use hifitime::{Duration, Epoch};

use crate::{
    engine::{QcEpochContext, QcProcessor},
    io::{
        input::indexing::QcIndexing,
        output::{QcOutput, QcOutputQueue, QcTecMapPoint, QcTecMapStatistics},
        source::QcTecPoint,
    },
    sync::{QcTecInterpolation, QcTecMap, QcTecStore},
};

/// Geographic region of a TEC map
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QcTecRegion {
    /// Name of the region
    pub name: String,

    /// Latitude range (°): (min, max), inclusive
    pub latitude_deg: (f64, f64),

    /// Longitude range (°): (min, max), inclusive. min > max crosses the antimeridian.
    pub longitude_deg: (f64, f64),
}

impl QcTecRegion {
    /// New region
    pub fn new(name: &str, latitude_deg: (f64, f64), longitude_deg: (f64, f64)) -> Self {
        Self {
            name: name.to_string(),
            latitude_deg,
            longitude_deg,
        }
    }

    /// Whole map
    pub fn global() -> Self {
        Self::new("Global", (-90.0, 90.0), (-180.0, 180.0))
    }

    /// Latitude band
    pub fn latitude_band(name: &str, min_deg: f64, max_deg: f64) -> Self {
        Self::new(name, (min_deg, max_deg), (-180.0, 180.0))
    }

    /// Global map, and five latitude bands (high, mid, equatorial latitudes)
    pub fn default_regions() -> Vec<Self> {
        vec![
            Self::global(),
            Self::latitude_band("North high latitudes", 60.0, 90.0),
            Self::latitude_band("North mid latitudes", 30.0, 60.0),
            Self::latitude_band("Equatorial", -30.0, 30.0),
            Self::latitude_band("South mid latitudes", -60.0, -30.0),
            Self::latitude_band("South high latitudes", -90.0, -60.0),
        ]
    }

    /// Returns true if these coordinates lie in this region
    pub fn contains(&self, latitude_deg: f64, longitude_deg: f64) -> bool {
        let (lat_min, lat_max) = self.latitude_deg;

        if latitude_deg < lat_min || latitude_deg > lat_max {
            return false;
        }

        let (lon_min, lon_max) = self.longitude_deg;

        if lon_max - lon_min >= 360.0 {
            return true;
        }

        let lon = (longitude_deg + 180.0).rem_euclid(360.0) - 180.0;
        let (min, max) = (
            (lon_min + 180.0).rem_euclid(360.0) - 180.0,
            (lon_max + 180.0).rem_euclid(360.0) - 180.0,
        );

        if min <= max {
            lon >= min && lon <= max
        } else {
            // antimeridian
            lon >= min || lon <= max
        }
    }
}

/// Output maps sampling
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QcTecSampling {
    /// The maps of the products, as is. Default.
    #[default]
    Native,

    /// One map every period, from the first map of each source: intermediate maps are
    /// interpolated in time.
    Period(Duration),

    /// The sampling interval of the products, divided by this factor
    /// (2.0: twice as many maps), from the first map of each source.
    Factor(f64),
}

/// [QcTecProcessor] configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QcTecConfig {
    /// Sources (agencies) to process, all of them by default
    pub sources: Option<Vec<QcIndexing>>,

    /// Output maps sampling
    pub sampling: QcTecSampling,

    /// Temporal interpolation strategy of the intermediate maps
    pub interpolation: QcTecInterpolation,

    /// Stream every node of every map (native or interpolated):
    /// allows the application to rebuild TEC maps (IONEX files for example)
    pub points: bool,

    /// Stream the statistics of every map, per region
    pub statistics: bool,

    /// Regions of the statistics
    pub regions: Vec<QcTecRegion>,
}

impl Default for QcTecConfig {
    fn default() -> Self {
        Self {
            sources: None,
            sampling: QcTecSampling::Native,
            interpolation: QcTecInterpolation::default(),
            points: false,
            statistics: true,
            regions: QcTecRegion::default_regions(),
        }
    }
}

impl QcTecConfig {
    /// One output map every period (temporal interpolation)
    pub fn with_sampling(mut self, period: Duration) -> Self {
        self.sampling = QcTecSampling::Period(period);
        self
    }

    /// Native sampling divided by this factor (temporal interpolation)
    pub fn with_upsampling_factor(mut self, factor: f64) -> Self {
        self.sampling = QcTecSampling::Factor(factor);
        self
    }

    /// Temporal interpolation strategy
    pub fn with_interpolation(mut self, interpolation: QcTecInterpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Streams every node of every map
    pub fn with_points(mut self) -> Self {
        self.points = true;
        self
    }

    /// Statistics regions
    pub fn with_regions(mut self, regions: Vec<QcTecRegion>) -> Self {
        self.regions = regions;
        self
    }

    /// Restricts to these sources
    pub fn with_sources(mut self, sources: &[QcIndexing]) -> Self {
        self.sources = Some(sources.to_vec());
        self
    }
}

#[derive(Debug, Clone, Default)]
struct SourceState {
    /// First map of the source: origin of the output grid
    origin: Option<Epoch>,
    /// Output period, once determined
    period: Option<Duration>,
    /// Latest output map
    last: Option<Epoch>,
}

/// Analyzes the TEC maps (IONEX), map after map: statistics per region
/// ([QcOutput::TecMap]), and possibly every node of every map ([QcOutput::TecPoint]).
/// Maps may be interpolated in time ([QcTecSampling]) to synthesize intermediate maps:
/// the applications accumulate the nodes to rebuild new TEC maps (IONEX files).
///
/// Results are time tagged with the map epoch. A map is produced once the surrounding
/// native maps are available, and its epoch has been reached by the engine.
#[derive(Debug, Clone, Default)]
pub struct QcTecProcessor {
    cfg: QcTecConfig,
    states: HashMap<QcIndexing, SourceState>,
}

impl QcTecProcessor {
    /// New [QcTecProcessor]
    pub fn new(cfg: QcTecConfig) -> Self {
        Self {
            cfg,
            states: Default::default(),
        }
    }

    /// Output map of `source` at `t`: (nodes, interpolated)
    fn output_map(
        &self,
        store: &QcTecStore,
        source: &QcIndexing,
        t: Epoch,
    ) -> Option<(Vec<QcTecPoint>, bool)> {
        if let Some(map) = store.map(source, t) {
            return Some((map.iter().collect(), false));
        }

        let ((_, grid), _) = store.bracket(source, t)?;

        // grid of the preceding map, interpolated in time
        let nodes = grid
            .iter()
            .filter_map(|node| {
                let value = store.tec_from(
                    source,
                    t,
                    node.latitude_deg,
                    node.longitude_deg,
                    Some(node.altitude_km),
                    self.cfg.interpolation,
                )?;

                Some(QcTecPoint {
                    tecu: value.tecu,
                    rms_tecu: value.rms_tecu,
                    ..node
                })
            })
            .collect::<Vec<_>>();

        Some((nodes, true))
    }

    fn statistics(
        &self,
        source: &QcIndexing,
        nodes: &[QcTecPoint],
        interpolated: bool,
    ) -> Vec<QcTecMapStatistics> {
        self.cfg
            .regions
            .iter()
            .filter_map(|region| {
                let selected = nodes
                    .iter()
                    .filter(|node| region.contains(node.latitude_deg, node.longitude_deg))
                    .collect::<Vec<_>>();

                if selected.is_empty() {
                    return None;
                }

                let n = selected.len() as f64;
                let mean = selected.iter().map(|node| node.tecu).sum::<f64>() / n;
                let var = selected
                    .iter()
                    .map(|node| (node.tecu - mean).powi(2))
                    .sum::<f64>()
                    / n;

                let min = selected
                    .iter()
                    .min_by(|a, b| a.tecu.partial_cmp(&b.tecu).unwrap())?;

                let max = selected
                    .iter()
                    .max_by(|a, b| a.tecu.partial_cmp(&b.tecu).unwrap())?;

                Some(QcTecMapStatistics {
                    source: source.clone(),
                    region: region.name.clone(),
                    interpolated,
                    nodes: selected.len(),
                    mean_tecu: mean,
                    std_tecu: var.sqrt(),
                    min_tecu: min.tecu,
                    max_tecu: max.tecu,
                    max_latitude_deg: max.latitude_deg,
                    max_longitude_deg: max.longitude_deg,
                })
            })
            .collect()
    }
}

/// Smallest interval between the first maps of the store
fn native_period(maps: &std::collections::BTreeMap<Epoch, QcTecMap>) -> Option<Duration> {
    let epochs = maps.keys().take(3).copied().collect::<Vec<_>>();
    epochs.windows(2).map(|w| w[1] - w[0]).min()
}

impl<O: From<QcOutput>> QcProcessor<O> for QcTecProcessor {
    fn name(&self) -> &str {
        "tec-maps"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>) {
        let store = ctx.tec_maps;

        let mut sources = store
            .sources()
            .filter(|source| {
                self.cfg
                    .sources
                    .as_ref()
                    .is_none_or(|selected| selected.contains(source))
            })
            .cloned()
            .collect::<Vec<_>>();

        sources.sort_by_key(|s| s.to_string());

        for source in sources {
            let maps = match store.maps(&source) {
                Some(maps) if !maps.is_empty() => maps,
                _ => continue,
            };

            let mut state = self.states.get(&source).cloned().unwrap_or_default();
            let first = *maps.keys().next().unwrap();
            let origin = *state.origin.get_or_insert(first);

            // output period
            if state.period.is_none() {
                state.period = match self.cfg.sampling {
                    QcTecSampling::Native => None,
                    QcTecSampling::Period(dt) => Some(dt),
                    QcTecSampling::Factor(factor) => {
                        if !(factor.is_finite() && factor > 0.0) {
                            continue;
                        }
                        match native_period(maps) {
                            Some(dt) => Some(dt / factor),
                            None => {
                                // a single map so far: wait for the next one
                                self.states.insert(source.clone(), state);
                                continue;
                            }
                        }
                    }
                };
            }

            // output epochs: (last, ctx.epoch]
            let epochs = match state.period {
                None => maps
                    .keys()
                    .filter(|e| state.last.is_none_or(|last| **e > last) && **e <= ctx.epoch)
                    .copied()
                    .collect::<Vec<_>>(),
                Some(period) if period > Duration::ZERO => {
                    let mut epochs = Vec::new();
                    let mut k = match state.last {
                        None => 0,
                        Some(last) => {
                            ((last - origin).to_seconds() / period.to_seconds()).round() as i64 + 1
                        }
                    };
                    // no extrapolation: up to the latest known map
                    let end = ctx.epoch.min(*maps.keys().next_back().unwrap());

                    loop {
                        let e = origin + period * k;
                        if e > end {
                            break;
                        }
                        epochs.push(e);
                        k += 1;
                    }
                    epochs
                }
                Some(_) => Vec::new(),
            };

            for e in epochs {
                // the surrounding maps must be known (no extrapolation)
                let Some((nodes, interpolated)) = self.output_map(store, &source, e) else {
                    continue;
                };

                state.last = Some(e);

                if self.cfg.statistics {
                    for stats in self.statistics(&source, &nodes, interpolated) {
                        output.push(e, QcOutput::TecMap(stats));
                    }
                }

                if self.cfg.points {
                    for point in nodes {
                        output.push(
                            e,
                            QcOutput::TecPoint(QcTecMapPoint {
                                source: source.clone(),
                                interpolated,
                                point,
                            }),
                        );
                    }
                }
            }

            self.states.insert(source, state);
        }
    }
}

#[cfg(test)]
mod test {
    use super::QcTecRegion;

    #[test]
    fn regions() {
        let global = QcTecRegion::global();
        assert!(global.contains(87.5, 180.0));

        let europe = QcTecRegion::new("Europe", (35.0, 70.0), (-10.0, 30.0));
        assert!(europe.contains(43.6, 1.44));
        assert!(!europe.contains(43.6, 40.0));
        assert!(!europe.contains(20.0, 1.44));

        // antimeridian
        let pacific = QcTecRegion::new("Pacific", (-30.0, 30.0), (150.0, -150.0));
        assert!(pacific.contains(0.0, 170.0));
        assert!(pacific.contains(0.0, -170.0));
        assert!(pacific.contains(0.0, 190.0));
        assert!(!pacific.contains(0.0, 0.0));
    }
}
