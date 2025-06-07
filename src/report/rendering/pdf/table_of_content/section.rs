use genpdf::Element;
use itertools::Itertools;

use crate::report::rendering::pdf::{
    PDF_LARGE_VERTICAL_SPACING, PDF_MEDIUM_VERTICAL_SPACING, PDF_MIN_VERTICAL_SPACING,
};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct QcPdfSection {
    pub name: String,
    pub paragraphs: Vec<String>,
}

impl QcPdfSection {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            paragraphs: Default::default(),
        }
    }

    pub fn add_paragraph(&mut self, paragraph: &str) {
        self.paragraphs.push(paragraph.to_string());
    }

    pub fn render(&self) -> genpdf::elements::LinearLayout {
        let mut layout = genpdf::elements::LinearLayout::vertical();
        layout.push(genpdf::elements::Paragraph::new(&self.name));

        for paragraph in self.paragraphs.iter().sorted() {
            layout.push(genpdf::elements::Break::new(PDF_MIN_VERTICAL_SPACING));

            layout.push(
                genpdf::elements::Paragraph::new(paragraph)
                    .styled(genpdf::style::Style::new().with_font_size(8)),
            );
        }

        layout
    }
}
