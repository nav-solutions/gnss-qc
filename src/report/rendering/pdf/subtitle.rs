use genpdf::Element;

pub struct QcPdfSubtitle {}

impl QcPdfSubtitle {
    pub fn new() -> genpdf::elements::StyledElement<genpdf::elements::Paragraph> {
        let mut paragraph = genpdf::elements::Paragraph::new("");

        paragraph.push(&format!("lib-gnss-qc v{}", env!("CARGO_PKG_VERSION"),));

        paragraph
            .aligned(genpdf::Alignment::Center)
            .styled(genpdf::style::Style::new().italic().with_font_size(10))
    }
}
