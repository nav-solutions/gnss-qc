use itertools::Itertools;
use maud::{html, Markup, Render};

use crate::report::{html::plot::Plot, QcRTKSummary};

use plotly::layout::MapboxStyle;

impl Render for QcRTKSummary {
    fn render(&self) -> Markup {
        let baselines_proj = Plot::world_map(
            "rtk-summary-baselines-proj",
            "Baselines projection",
            MapboxStyle::OpenStreetMap,
            (0.0, 0.0),
            1,
            true,
        );

        html! {

            div class="styled-table" {
                table class="table is-bordered" {
                    tr {
                        th {
                            "Base Network Baselines Projection (km)"
                        }

                        @ for base in self.base_network_distances_km.keys().map(|(base_i, _)| base_i).unique().sorted() {
                            td {
                                (base)
                            }
                        }

                        @ for (i, base_i) in self.base_network_distances_km.keys().map(|(base_i, _)| base_i).unique().sorted().enumerate() {
                            tr {
                                td {
                                    (base_i)
                                }

                               @ for (j, base_j) in self.base_network_distances_km.keys().map(|(_, base_j)| base_i).unique().sorted().enumerate() {
                                td {
                                    ("0.0")
                                }
                               }
                            }
                        }
                    }

                    tr {
                        th {
                            "Rover/Base Baselines Projections (km)"
                        }

                        @ for (ith_base, base) in self.baseline_distances_km.keys().map(|(base, _)| base).unique().sorted().enumerate() {
                            td {
                                (base)
                            }
                        }

                        @ for (ith_rover, rover) in self.baseline_distances_km.keys().map(|(_, rover)| rover).unique().sorted().enumerate() {
                            tr {
                                td {
                                    (rover)
                                }
                                @ for (ith_base, base) in self.baseline_distances_km.keys().map(|(base, _)| base).unique().sorted().enumerate() {
                                    td {
                                        (base)
                                    }
                                }
                            }
                        }

                    }
                }
            }
        }
    }
}
