use genpdf::{Element, RenderResult};

pub struct QcPdfDocumentation {}

impl QcPdfDocumentation {
    pub fn new() -> Self {
        Self {}
    }
}

impl Element for QcPdfDocumentation {
    fn render(
        &mut self,
        context: &genpdf::Context,
        area: genpdf::render::Area<'_>,
        style: genpdf::style::Style,
    ) -> Result<RenderResult, genpdf::error::Error> {
        let mut layout = genpdf::elements::LinearLayout::vertical();
        layout.push(genpdf::elements::Paragraph::new("Documentation"));
        layout.render(context, area, style)
    }
}
