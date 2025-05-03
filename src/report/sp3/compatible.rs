use itertools::Itertools;
use maud::{html, Markup, Render};

use sp3::prelude::{Constellation, SP3, SV};

use crate::report::shared::SamplingReport;

pub struct FileReport {
    agency: String,
    revision: String,
    coord_system: String,
    orbit_fit: String,
    time_scale: String,
    has_clock: bool,
    has_velocity: bool,
    has_clock_drift: bool,
    uses_prediction: bool,
    has_maneuvers: bool,
    has_clock_events: bool,
    satellites: Vec<SV>,
    sampling: SamplingReport,
    constellations: Vec<Constellation>,
}

impl FileReport {
    pub fn new(sp3: &SP3) -> Self {
        let satellites = sp3.satellites_iter().collect::<Vec<_>>();

        Self {
            agency: sp3.header.agency.clone(),
            revision: sp3.header.version.to_string().to_uppercase(),
            coord_system: sp3.header.coord_system.clone(),
            orbit_fit: sp3.header.orbit_type.to_string(),
            time_scale: sp3.header.timescale.to_string(),
            has_clock: sp3.has_satellite_clock_offset(),
            has_velocity: sp3.has_satellite_velocity(),
            has_clock_drift: sp3.has_satellite_clock_drift(),
            has_clock_events: sp3.has_satellite_clock_event(),
            uses_prediction: sp3.has_satellite_positions_prediction(),
            has_maneuvers: sp3.has_satellite_maneuver(),
            sampling: SamplingReport::from_sp3(sp3),
            satellites,
            // constellations: satellites
            //     .iter()
            //     .map(|sv| sv.constellation)
            //     .unique()
            //     .collect(),
            constellations: Default::default(),
        }
    }
}

impl Render for FileReport {
    fn render(&self) -> Markup {
        html! {
            div class="styled-table" {
                table class="table is-bordered" {
                    tr {
                        th {
                            "Agency"
                        }
                        td {
                            (self.agency)
                        }
                    }
                    tr {
                        th {
                            "Revision"
                        }
                        td {
                            (self.revision)
                        }
                    }
                    tr {
                        th {
                            "Timescale"
                        }
                        td {
                            (self.time_scale)
                        }
                    }
                    tr {
                        th {
                            "Reference Frame"
                        }
                        td {
                            (self.coord_system)
                        }
                    }
                    tr {
                        th {
                            "Fit Algorithm"
                        }
                        td {
                            (self.orbit_fit)
                        }
                    }
                    tr {
                        th {
                            "Has Precise Velocity"
                        }
                        td {
                            (self.has_velocity)
                        }
                    }
                    tr {
                        th {
                            "Has Precise Clock"
                        }
                        td {
                            (self.has_clock)
                        }
                    }
                    tr {
                        th {
                            "Has Precise Clock Drift"
                        }
                        td {
                            (self.has_clock_drift)
                        }
                    }
                    tr {
                        th {
                            "Predicted positions"
                        }
                        td {
                            (self.uses_prediction)
                        }
                    }
                    tr {
                        th {
                            "Has Precise Clock Drift"
                        }
                        td {
                            (self.has_clock_drift)
                        }
                    }
                    tr {
                        th {
                            "SV Maneuvers"
                        }
                        td {
                            (self.has_maneuvers)
                        }
                    }
                    tr {
                        th {
                            "SV Clock Events"
                        }
                        td {
                            (self.has_clock_events)
                        }
                    }
                    tr {
                        th class="is-info" {
                            "Constellations"
                        }
                        td {
                            (self.constellations.iter().sorted().join(", "))
                        }
                    }
                    tr {
                        th class="is-info" {
                            "Satellites"
                        }
                        td {
                            (self.satellites.iter().sorted().join(", "))
                        }
                    }
                    tr {
                        th class="is-info" {
                            "Sampling"
                        }
                        td {
                            (self.sampling.render())
                        }
                    }
                }
            }
        }
    }
}
