use itertools::Itertools;
use maud::{html, Markup, Render};

use crate::report::QcRTKSummary;

impl Render for QcRTKSummary {
    fn render(&self) -> Markup {
        html! {

            div class="styled-table" {
                table class="table is-bordered" {
                    tr {
                        th {
                            "Rovers"
                        }
                    }
                    @ for rover in self.rovers.keys().sorted() {
                        tr {
                            td {}
                            td {
                                (rover)
                            }
                        }
                    }
                    tr {
                        th {
                            "Bases"
                        }
                    }
                    @ for base in self.bases.keys().sorted() {
                        tr {
                            td {}
                            td {
                                (base)
                            }
                        }
                    }
                    tr {
                        th {
                            "Baselines (km)"
                        }
                    }
                    @ for (rover, base) in self.baselines.keys().sorted() {
                        @ if let Some(baseline) = self.baselines.get(&(rover.clone(), base.clone())) {
                            tr {
                                th {
                                    (format!("{}/{}", rover, base))
                                }
                                td {
                                    (format!("{}", baseline *1e-3))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
