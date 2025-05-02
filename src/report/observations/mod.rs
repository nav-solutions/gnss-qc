use crate::{
    context::{QcContext, QcIndexing},
    prelude::{html, Markup, Render, Rinex, TimeScale},
};

mod bias;
use bias::BiasSummary;

pub struct Report {
    /// [TimeScale] observations are expressed in
    timescale: TimeScale,

    /// [BiasSummary]
    bias: BiasSummary,
}

impl Report {
    pub fn new(ctx: &QcContext, source: &QcIndexing, rinex: &Rinex) -> Self {
        Self {
            bias: BiasSummary::new(ctx, rinex),
            timescale: TimeScale::GPST,
        }
    }
}

impl Render for Report {
    fn render(&self) -> Markup {
        html! {}
    }
}
// tr {
//     th {
//         button aria-label="Timescale in which observations are expressed.
// Navigation solutions are expressed in this timescale by default." data-balloon-pos="right" {
//             "Timescale"
//         }
//     }
//     @if let Some(timescale) = self.timescale {
//         td {
//             (timescale.to_string())
//         }
//     } @else {
//         td {
//             button aria-label="This dataset is not a timeserie." data-balloon-pos="up" {
//                 "Not Applicable"
//             }
//         }
//     }
// }
