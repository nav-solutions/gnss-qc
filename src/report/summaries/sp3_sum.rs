use sp3::prelude::{Header, TimeScale, Version};

/// [QcSP3FileSummary] summary.
#[derive(Debug, Clone, Default)]
pub struct QcSP3FileSummary {
    /// SP3 [Version]
    pub version: Version,

    /// [TimeScale]
    pub timescale: TimeScale,

    /// Agency
    pub agency: String,

    /// Coordinates system
    pub frame: String,

    /// Orbit type
    pub orbit_type: String,

    pub has_sv_velocities: bool,
    pub has_sv_clock_offsets: bool,
    pub has_sv_clock_drift: bool,
    pub has_sv_clock_event: bool,
    pub has_sv_maneuver: bool,
}

impl QcSP3FileSummary {
    pub fn from_header(header: &Header) -> Self {
        Self {
            version: header.version,
            frame: header.coord_system.clone(),
            timescale: header.timescale,
            agency: header.agency.clone(),
            orbit_type: header.orbit_type.to_string(),

            has_sv_clock_drift: false,
            has_sv_clock_event: false,
            has_sv_clock_offsets: false,
            has_sv_maneuver: false,
            has_sv_velocities: false,
        }
    }
}

#[cfg(feature = "html")]
use maud::{html, Markup, Render};

#[cfg(feature = "html")]
impl Render for QcSP3FileSummary {
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
                            (self.version)
                        }
                    }
                    tr {
                        th {
                            "Reference Frame"
                        }
                        td {
                            (self.frame)
                        }
                    }
                    tr {
                        th {
                            "Timescale"
                        }
                        td {
                            (self.timescale)
                        }
                    }
                    tr {
                        th {
                            "Orbit Type"
                        }
                        td {
                            (self.orbit_type)
                        }
                    }
                    tr {
                        th {
                            "Velocities"
                        }
                    }
                    tr {
                        th {
                            "Clock Offset"
                        }
                    }
                    tr {
                        th {
                            "Clock drift"
                        }
                    }
                    tr {
                        th {
                            "Satellite maneuver"
                        }
                    }
                    tr {
                        th {
                            "Clock event (bumps)"
                        }
                    }
                }
            }
        }
    }
}
