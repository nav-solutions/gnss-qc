use crate::report::rendering::pdf::PDF_LARGE_VERTICAL_SPACING;

use super::PDF_MIN_VERTICAL_SPACING;

use genpdf::Element;

pub struct QcPdfCredits {}

impl QcPdfCredits {
    pub fn new() -> Self {
        Self {}
    }
}

impl Element for QcPdfCredits {
    fn render(
        &mut self,
        context: &genpdf::Context,
        area: genpdf::render::Area<'_>,
        style: genpdf::style::Style,
    ) -> Result<genpdf::RenderResult, genpdf::error::Error> {
        let mut layout = genpdf::elements::LinearLayout::vertical();
        layout.push(genpdf::elements::Paragraph::new("Credits"));
        layout.render(context, area, style)
    }
}
