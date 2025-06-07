use itertools::Itertools;

use crate::report::summaries::{QcContextSummary, QcFileSummary};

use genpdf::{error::Error, render::Area, style::Style, Context, Element, Mm, RenderResult, Size};

impl Element for QcContextSummary {
    fn render(
        &mut self,
        context: &Context,
        area: Area<'_>,
        style: Style,
    ) -> Result<RenderResult, Error> {
        Ok(RenderResult {
            size: Size {
                width: Mm::default(),
                height: Mm::default(),
            },
            has_more: false,
        })
    }
}
