use super::section::QcPdfSection;
use crate::report::rendering::pdf::{
    PDF_LARGE_VERTICAL_SPACING, PDF_MEDIUM_VERTICAL_SPACING, PDF_MIN_VERTICAL_SPACING,
};

use genpdf::Element;
use itertools::Itertools;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct QcPdfChapter {
    pub name: String,
    pub sections: Vec<QcPdfSection>,
}

impl QcPdfChapter {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            sections: Default::default(),
        }
    }

    pub fn add_section(&mut self, section: QcPdfSection) {
        self.sections.push(section);
    }

    pub fn render(&self) -> genpdf::elements::LinearLayout {
        let mut layout = genpdf::elements::LinearLayout::vertical();
        layout.push(genpdf::elements::Paragraph::new(&self.name));

        for section in self.sections.iter().sorted() {
            layout.push(genpdf::elements::Break::new(PDF_MIN_VERTICAL_SPACING));
            layout.push(
                section
                    .render()
                    .styled(genpdf::style::Style::new().bold().with_font_size(10)),
            );
        }

        layout
    }
}
