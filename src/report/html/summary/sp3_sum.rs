use maud::{html, Markup, Render};

use crate::report::summaries::sp3_sum::QcSP3FileSummary;

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
                        td {
                            (self.has_sv_velocities)
                        }
                    }
                    tr {
                        th {
                            "Clock Offset"
                        }
                        td {
                            (self.has_sv_clock_offsets)
                        }
                    }
                    tr {
                        th {
                            "Clock drift"
                        }
                        td {
                            (self.has_sv_clock_drift)
                        }
                    }
                    tr {
                        th {
                            "Satellite maneuver"
                        }
                        td {
                            (self.has_sv_maneuver)
                        }
                    }
                    tr {
                        th {
                            "Clock events (bumps)"
                        }
                        td {
                            (self.has_sv_clock_event)
                        }
                    }
                }
            }
        }
    }
}
