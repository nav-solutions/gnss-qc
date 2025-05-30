use itertools::Itertools;
use maud::{html, Markup, Render};

use crate::report::{html::plot::Plot, QcRTKSummary};

use plotly::{
    common::{color::NamedColor, MarkerSymbol},
    layout::MapboxStyle,
};

impl Render for QcRTKSummary {
    fn render(&self) -> Markup {
        let mut center_ddeg = (0.0, 0.0);
        let mut traces = vec![];

        // draw all base stations
        for (base_label, position) in self.bases.iter() {
            if let Some(position) = position {
                match position.to_earth_geodetic_degrees_km() {
                    Ok((lat_ddeg, long_ddeg, _)) => {
                        let tr = Plot::mapbox(
                            vec![lat_ddeg],
                            vec![long_ddeg],
                            base_label,
                            3,
                            MarkerSymbol::Circle,
                            Some(NamedColor::Black),
                            1.0,
                            true,
                        );

                        center_ddeg = (lat_ddeg, long_ddeg);
                        traces.push(tr);
                    }
                    Err(e) => {
                        println!("error={}", e);
                    }
                }
            }
        }

        // draw all rovers
        for (rover_label, position) in self.rovers.iter() {
            if let Some(position) = position {
                match position.to_earth_geodetic_degrees_km() {
                    Ok((lat_ddeg, long_ddeg, _)) => {
                        let tr = Plot::mapbox(
                            vec![lat_ddeg],
                            vec![long_ddeg],
                            rover_label,
                            3,
                            MarkerSymbol::Circle,
                            Some(NamedColor::Black),
                            1.0,
                            true,
                        );

                        center_ddeg = (lat_ddeg, long_ddeg);
                        traces.push(tr);
                    }
                    Err(e) => {
                        println!("error={}", e);
                    }
                }
            }
        }

        // add base_i-base_j baselines
        for (base_i, base_j) in self.base_network_distances_km.keys().unique().sorted() {}

        let mut map = Plot::world_map(
            "rtk-summary-baselines-proj",
            "Baselines projection",
            MapboxStyle::OpenStreetMap,
            center_ddeg,
            18,
            true,
        );

        for trace in traces {
            panic!("test");
            map.add_trace(trace);
        }

        html! {
            div class="styled-table" {
                table class="table is-bordered" {
                    tr {
                        th {
                            "Base Network Distances (km)"
                        }

                        @ for (base_i, base_j) in self.base_network_distances_km.keys().unique().sorted() {
                            tr {
                                td {
                                    (format!("{}/{}", base_i, base_j))
                                }
                            }
                        }
                    }
                    tr {
                        (map)
                    }
                }
            }
        }
    }
}
