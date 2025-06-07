use genpdf::{Element, RenderResult};

use crate::{context::QcProductType, report::QcRunReport};

use crate::report::rendering::pdf::PDF_LARGE_VERTICAL_SPACING;

pub mod section;
use section::QcPdfSection;

mod chapter;
use chapter::QcPdfChapter;

use super::{vertical_separator::QcPdfVerticalSeparator, PDF_MIN_VERTICAL_SPACING};

pub struct QcPdfTableOfContent {
    pub chapters: Vec<QcPdfChapter>,
}

impl Element for QcPdfTableOfContent {
    fn render(
        &mut self,
        context: &genpdf::Context,
        area: genpdf::render::Area<'_>,
        style: genpdf::style::Style,
    ) -> Result<RenderResult, genpdf::error::Error> {
        let mut layout = genpdf::elements::LinearLayout::vertical();

        layout.push(genpdf::elements::Break::new(PDF_LARGE_VERTICAL_SPACING));

        let mut title_layout = genpdf::elements::LinearLayout::vertical();
        title_layout.push(genpdf::elements::Paragraph::new("Table of Content"));
        layout.push(title_layout.styled(genpdf::style::Style::new().bold().with_font_size(20)));

        for chapter in self.chapters.iter() {
            layout.push(genpdf::elements::Break::new(PDF_MIN_VERTICAL_SPACING));
            layout.push(QcPdfVerticalSeparator::new().render());
            layout.push(
                chapter
                    .render()
                    .styled(genpdf::style::Style::new().bold().with_font_size(15)),
            );
        }

        layout.render(context, area, style)
    }
}

impl QcPdfTableOfContent {
    pub fn new(report: &QcRunReport) -> Self {
        let mut chapters = Vec::new();

        // Summary
        let mut chapter = QcPdfChapter::new("Summary");
        chapter.add_section(QcPdfSection::new("Run summary"));

        if let Some(summary) = &report.summary {
            let has_rinex = summary
                .summaries
                .keys()
                .filter(|source| source.product_type.is_rinex_product())
                .count()
                > 0;

            if has_rinex {
                let mut section = QcPdfSection::new("RINEX");

                for file in summary.summaries.keys().filter_map(|source| {
                    if source.product_type.is_rinex_product() {
                        Some(&source.filename)
                    } else {
                        None
                    }
                }) {
                    section.add_paragraph(file);
                }

                chapter.add_section(section);
            }

            let has_sp3 = summary
                .summaries
                .keys()
                .filter(|source| source.product_type == QcProductType::PreciseOrbit)
                .count()
                > 0;

            if has_sp3 {
                let mut section = QcPdfSection::new("SP3");

                for file in summary.summaries.keys().filter_map(|source| {
                    if source.product_type == QcProductType::PreciseOrbit {
                        Some(&source.filename)
                    } else {
                        None
                    }
                }) {
                    section.add_paragraph(file);
                }

                chapter.add_section(section);
            }
        }

        chapters.push(chapter);

        // RTK-summary
        chapters.push(QcPdfChapter::new("RTK-Summary"));

        // Observations
        chapters.push(QcPdfChapter::new("Observations"));

        let mut documentation = QcPdfChapter::new("Documentation");

        let mut section = QcPdfSection::new("Framework");
        section.add_paragraph("github.com");
        section.add_paragraph("GNSS-Qc (API)");
        section.add_paragraph("RINEX (API)");
        section.add_paragraph("SP3 (API)");

        documentation.add_section(section);

        chapters.push(documentation);

        let mut credits = QcPdfChapter::new("Credits");
        chapters.push(credits);

        Self { chapters }
    }
}
