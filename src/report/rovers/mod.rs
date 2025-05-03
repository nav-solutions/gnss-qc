use crate::{
    context::{QcContext, QcIndexing},
    prelude::{html, Markup, Render},
    report::selector::Selector,
};

use std::collections::HashMap;

mod rover;
use rover::Report as RoverReport;

/// Rovers report (one for each)
pub struct Report {
    selector: Selector<QcIndexing>,
    rovers: HashMap<QcIndexing, RoverReport>,
}

impl Default for Report {
    fn default() -> Self {
        Self {
            selector: Selector::new("rovers"),
            rovers: Default::default(),
        }
    }
}

impl Report {
    pub fn has_content(&self) -> bool {
        self.selector.has_content()
    }

    pub fn new(ctx: &QcContext) -> Self {
        let mut selector = Selector::new("rovers");
        let mut rovers = HashMap::new();

        for rover in ctx.observations.keys() {
            selector.add(rover);
            rovers.insert(rover.clone(), RoverReport::new(ctx, rover));
        }

        Self { rovers, selector }
    }
}

impl Render for Report {
    fn render(&self) -> Markup {
        html! {}
    }
}
