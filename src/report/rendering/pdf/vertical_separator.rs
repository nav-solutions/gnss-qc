pub struct QcPdfVerticalSeparator {}

impl QcPdfVerticalSeparator {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&self) -> genpdf::elements::LinearLayout {
        let mut layout = genpdf::elements::LinearLayout::vertical();
        layout.push(genpdf::elements::Paragraph::new(
            "___________________________________________________",
        ));
        layout.push(genpdf::elements::Break::new(1.0));
        layout
    }
}
