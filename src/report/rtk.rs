use std::{collections::HashMap, thread::park_timeout_ms};

use anise::time;

use crate::{prelude::QcReferencePosition, serializer::data::QcSerializedRINEXHeader};

#[derive(Clone, Default)]
pub struct QcRTKSummary {
    /// Rovers network.
    pub rovers: HashMap<String, Option<QcReferencePosition>>,

    /// Bases network.
    pub bases: HashMap<String, Option<QcReferencePosition>>,

    /// Baselines: projected distance (km) between all bases and reovers in the Network.
    pub baseline_distances_km: HashMap<(String, String), f64>,

    /// Projected distance (km) between all bases in Network.
    pub base_network_distances_km: HashMap<(String, String), f64>,
}

impl QcRTKSummary {
    /// Initializes a [QcRTKSummary] from ROVER [QcSerializedRINEXHeader].
    pub fn from_rover_header(item: QcSerializedRINEXHeader) -> Self {
        Self {
            rovers: {
                let mut map = HashMap::new();

                if let Some(position) = item.data.rx_position {
                    if let Some(obs) = item.data.obs {
                        if let Some(time_of_first_obs) = obs.timeof_first_obs {
                            let position = QcReferencePosition::new(position, time_of_first_obs);

                            map.insert(item.indexing.to_string(), Some(position));
                        }
                    }
                }

                if map.is_empty() {
                    map.insert(item.indexing.to_string(), None);
                }

                map
            },
            bases: Default::default(),
            baseline_distances_km: Default::default(),
            base_network_distances_km: Default::default(),
        }
    }

    /// Initializes a [QcRTKSummary] from BASE [QcSerializedRINEXHeader].
    pub fn from_base_header(item: QcSerializedRINEXHeader) -> Self {
        Self {
            bases: {
                let mut map = HashMap::new();

                if let Some(position) = item.data.rx_position {
                    if let Some(obs) = item.data.obs {
                        if let Some(time_of_first_obs) = obs.timeof_first_obs {
                            let position = QcReferencePosition::new(position, time_of_first_obs);
                            map.insert(item.indexing.to_string(), Some(position));
                        }
                    }
                }

                if map.is_empty() {
                    map.insert(item.indexing.to_string(), None);
                }

                map
            },
            rovers: Default::default(),
            baseline_distances_km: Default::default(),
            base_network_distances_km: Default::default(),
        }
    }

    /// Latch a new ROVER [QcSerializedRINEXHeader].
    pub fn latch_rover_header(&mut self, item: QcSerializedRINEXHeader) {
        if let Some(position) = item.data.rx_position {
            if let Some(obs) = item.data.obs {
                if let Some(time_of_first_obs) = obs.timeof_first_obs {
                    let position = QcReferencePosition::new(position, time_of_first_obs);

                    self.rovers
                        .insert(item.indexing.to_string(), Some(position));

                    // add new baseline combinations
                    for (base_name, base_pos) in self.bases.iter() {
                        if let Some(base_position) = base_pos {
                            let dist = 0.0;
                            self.baseline_distances_km
                                .insert((base_name.clone(), item.indexing.to_string()), dist);
                        }
                    }
                }
            }
        } else {
            self.rovers.insert(item.indexing.to_string(), None);
        }
    }

    /// Latch a new BASE [QcSerializedRINEXHeader].
    pub fn latch_base_header(&mut self, item: QcSerializedRINEXHeader) {
        let base_name = item.indexing.to_string();

        if let Some(position) = item.data.rx_position {
            if let Some(obs) = item.data.obs {
                if let Some(time_of_first_obs) = obs.timeof_first_obs {
                    let position = QcReferencePosition::new(position, time_of_first_obs);

                    self.bases.insert(item.indexing.to_string(), Some(position));

                    // add new baseline combination
                    for (rover_name, rover_pos) in self.rovers.iter() {
                        if let Some(rover_position) = rover_pos {
                            let dist = 0.0;

                            self.baseline_distances_km
                                .insert((item.indexing.to_string(), rover_name.clone()), dist);
                        }
                    }

                    // add new |base - base| projection
                    for (name, base_pos) in self.bases.iter() {
                        if name != &base_name {
                            if let Some(base_position) = base_pos {
                                let dist = 0.0;

                                self.base_network_distances_km
                                    .insert((name.to_string(), base_name.clone()), 0.0);
                            }
                        }
                    }
                }
            }
        } else {
            self.bases.insert(base_name, None);
        }
    }
}
