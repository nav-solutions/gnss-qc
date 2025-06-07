use itertools::Itertools;
use maud::{html, Markup, Render};

use crate::report::summaries::rinex_sum::QcRINEXFileSummary;

impl Render for QcRINEXFileSummary {
    fn render(&self) -> Markup {
        html! {
            div class="styled-table" {
                table class="table is-bordered" {
                    tr {
                        th {
                            "Revision"
                        }
                        td {
                            (format!("v{}", self.version))
                        }
                    }
                    tr {
                        th {
                            "Timescale"
                        }
                        td {
                            @ if let Some(timescale) = self.timescale {
                                (timescale)
                            } @ else {
                                "Undefined"
                            }
                        }
                    }
                    @ if let Some(reference_position) = &self.reference_position {
                        tr {
                            th {
                                "Reference position"
                            }
                            td {
                                (reference_position.render())
                            }
                        }
                    }
                    @ if let Some(receiver) = &self.receiver {
                        tr {
                            th {
                                "Receiver"
                            }
                            tr {
                                th {
                                    "  Model"
                                }
                                td {
                                    (receiver.model)
                                }
                            }
                        }
                    }
                    @ if let Some(antenna) = &self.antenna {
                        tr {
                            th {
                                "Antenna"
                            }
                            tr {
                                th {
                                    "  Model"
                                }
                                td {
                                    (antenna.model)
                                }
                            }
                        }
                    }
                    @ if let Some(marker) = &self.geodetic_marker {
                        tr {
                            th {
                                "Geodetic Marker"
                            }
                            tr {
                                th {
                                    "  Name"
                                }
                                td {
                                    (marker.name)
                                }
                            }
                        }
                    }
                    @ if !self.v3_time_corrections.is_empty() {
                        tr {
                            th {
                                "V3 System Time Corrections"
                            }
                            td {
                                @ for (lhs, rhs) in self.v3_time_corrections.iter().sorted() {
                                    (format!("{}/{}, ", lhs, rhs))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
