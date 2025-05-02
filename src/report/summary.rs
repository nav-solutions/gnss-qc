use hifitime::Unit;
use maud::{html, Markup, Render};

use crate::prelude::{Epoch, QcConfig, QcContext};

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
    /// Synthesis datetime as [Epoch]
    datetime: Epoch,

    /// Configuration used
    cfg: QcConfig,
}

impl QcSummary {
    /// Generate a new [QcSummary]
    /// ## Input
    /// - now: [Epoch] of synthesis
    /// - context: current [QcContext] being reported
    pub fn new(now: Epoch, context: &QcContext) -> Self {
        Self {
            datetime: now.round(10.0 * Unit::Second),
            cfg: context.configuration.clone(),
        }
    }
}

impl Render for QcSummary {
    fn render(&self) -> Markup {
        html! {
            table class="styled-table" {
                tbody {
                    tr {
                        th class="is-info is-bordered" {
                            "Date/Time"
                        }
                        td {
                            (self.datetime.to_string())
                        }
                    }
                    tr {
                        th class="is-info is-bordered" {
                            "Configuration"
                        }
                        td {
                            (self.cfg.render())
                        }
                    }
                }
            }
        }
    }
}
