use crate::{
    prelude::{Duration, Epoch},
    processing::analysis::QcAnalysis,
};

/// [QcPipeline] run summary.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct QcRunSummary {
    /// Deployment datetime, as [Epoch]
    pub datetime: Epoch,

    /// Total processing time, as [Duration]
    pub run_duration: Duration,

    /// Analysis that were selected
    pub analysis: Vec<QcAnalysis>,
}

use maud::{html, Markup, Render};

impl Render for QcRunSummary {
    fn render(&self) -> Markup {
        html! {
            div class="styled-table" {
                table class="table is-bordered" {
                    tr {
                        th {
                            "Deployment time"
                        }
                        td {
                            (self.datetime.to_string())
                        }
                    }
                    tr {
                        th {
                            "Run duration"
                        }
                        td {
                            (self.run_duration.to_string())
                        }
                    }
                    tr {
                        th {
                            "Analysis"
                        }
                        td {
                            div class="styled-table" {
                                table class="table is-bordered" {
                                    @ for analysis in self.analysis.iter() {
                                        tr {
                                            td {
                                                (analysis.to_string())
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
    }
}
