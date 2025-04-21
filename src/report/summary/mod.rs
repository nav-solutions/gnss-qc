use maud::{html, Markup, Render};
use rinex::prelude::TimeScale;

use crate::prelude::{QcConfig, QcContext};

mod nav_post;
use nav_post::QcNavPostSummary;

mod bias;
use bias::QcBiasSummary;

/// Although simplistic, [QcSummary] is very powerful and gives
/// meaningful information. In particular:
///
/// - A unique identification for this session.
/// This allows differentiating sessions.
/// - The [TimeScale] that applies. When signals were loaded,
/// this is the [TimeScale] in which they were expressed in.
/// - The [QcNavPostSummary] describes post processed navigation capabilities.
/// In short, what you can achieve using the provided setup.
/// - Other meaningful information, like bias cancelling capabilities,
/// once again useful in post processed navigation.
#[derive(Clone)]
pub struct QcSummary {
    name: String,
    /// Configuration used
    cfg: QcConfig,
    /// NAVI summary
    pub navi: QcNavPostSummary,
    /// Main timescale
    timescale: Option<TimeScale>,
    /// BIAS summary
    bias_sum: QcBiasSummary,
}

impl QcSummary {
    pub fn new(context: &QcContext) -> Self {
        Self {
            name: context.name(),
            timescale: context.timescale(),
            bias_sum: QcBiasSummary::new(context),
            navi: QcNavPostSummary::new(context),
            cfg: context.configuration.clone(),
        }
    }
}

impl Render for QcSummary {
    fn render(&self) -> Markup {
        html! {
            div class="table-container" {
                table class="table is-bordered" {
                    tbody {
                        tr {
                            th class="is-info is-bordered" {
                                (self.name.clone())
                            }
                        }
                        tr {
                            th {
                                button aria-label="Timescale in which observations are expressed.
        Navigation solutions are expressed in this timescale by default." data-balloon-pos="right" {
                                    "Timescale"
                                }
                            }
                            @if let Some(timescale) = self.timescale {
                                td {
                                    (timescale.to_string())
                                }
                            } @else {
                                td {
                                    button aria-label="This dataset is not a timeserie." data-balloon-pos="up" {
                                        "Not Applicable"
                                    }
                                }
                            }
                        }
                        tr {
                            th class="is-info" {
                                button aria-label="Context / Dataset compliancy" data-balloon-pos="right" {
                                    "Compliancy"
                                }
                            }
                            td {
                                (self.navi.render())
                            }
                        }
                        tr {
                            th class="is-info" {
                                button aria-label="Physical and Environmental bias analysis & cancellation capabilities" data-balloon-pos="right" {
                                    "Bias"
                                }
                            }
                            td {
                                (self.bias_sum.render())
                            }
                        }
                    }
                }
            }
        }
    }
}
