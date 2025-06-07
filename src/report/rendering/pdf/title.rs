use genpdf::Element;

pub struct QcPdfTitle {}

impl QcPdfTitle {
    pub fn new() -> genpdf::elements::StyledElement<genpdf::elements::Paragraph> {
        let mut paragraph = genpdf::elements::Paragraph::new("GNSS Quality Control Report")
            .aligned(genpdf::Alignment::Center)
            .styled(genpdf::style::Style::new().bold().with_font_size(20));

        paragraph
    }
}
