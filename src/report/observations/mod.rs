use crate::{
    context::{QcContext, QcIndexing},
    prelude::{html, Markup, Render, Rinex, TimeScale},
};

mod bias;
use bias::BiasSummary;

pub struct Report {
    /// [TimeScale] observations are expressed in
    timescale: Option<TimeScale>,

    /// [BiasSummary]
    bias: BiasSummary,
}

impl Report {
    pub fn new(ctx: &QcContext, source: &QcIndexing, rinex: &Rinex) -> Self {
        Self {
            bias: BiasSummary::new(ctx, rinex),
            timescale: ctx.observations_timescale(source),
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
