use crate::prelude::{html, Markup, QcContext, QcIndexing, Render, TimeScale};

mod bias;
use bias::BiasSummary;

// mod orbital_proj;
// use orbital_proj::Projection as OrbitalProjection;

// mod observations;
// use observations::Report as ObservationsReport;

pub struct Report {
    bias: BiasSummary,
    timescale: Option<TimeScale>,
    // orbit_proj: OrbitalProjection,
    // observations: ObservationsReport,
}

impl Report {
    pub fn new(ctx: &QcContext, source: &QcIndexing) -> Self {
        let observations = ctx
            .observations_data(source)
            .expect("internal error: data should exist");

        Self {
            bias: BiasSummary::new(ctx, observations),
            timescale: ctx.observations_timescale(source),
            // orbit_proj: OrbitalProjection::new(&ctx, &observations),
            // observations: ObservationsReport::new(&observations),
        }
    }
}

impl Render for Report {
    fn render(&self) -> Markup {
        html! {
            table class="styled-table" {
                tbody {
                    tr {
                        th {
                            button aria-label="Timescale in which observations are expressed.
                    Navigation solutions are expressed in this timescale by default." data-balloon-pos="right" {
                                "Timescale"
                            }
                        }
                        @ if let Some(timescale) = &self.timescale {
                            td {
                                (timescale.to_string())
                            }
                        } @ else {
                            td {
                                "Undefined"
                            }
                        }
                    }
                    tr {
                        th {
                            "Bias"
                        }
                        td {
                            (self.bias.render())
                        }
                    }
                }
            }
        }
    }
}
