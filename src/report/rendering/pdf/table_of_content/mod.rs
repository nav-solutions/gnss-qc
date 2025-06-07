use itertools::Itertools;

use genpdf::{Element, RenderResult};

use crate::{context::QcProductType, report::QcRunReport};

use std::collections::HashMap;

use crate::report::rendering::pdf::{PDF_LARGE_VERTICAL_SPACING, PDF_MEDIUM_VERTICAL_SPACING};

pub mod section;
use section::QcPdfSection;

mod chapter;
use chapter::QcPdfChapter;

use super::PDF_MIN_VERTICAL_SPACING;

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
        chapter.add_section(QcPdfSection::new("Run report"));

        if let Some(summary) = &report.summary {
            let has_rinex = summary
                .summaries
                .keys()
                .filter(|source| source.product_type.is_rinex_product())
                .count()
                > 0;

            if has_rinex {
                chapter.add_section(QcPdfSection::new("RINEX"));

                // let sum = chapters.get_mut("Summary").unwrap();
                // sum.push("RINEX".to_string());

                // for name in summary.summaries.keys().sorted() {
                //     sum.push(format!("{}({})", name.filename, name.product_type));
                // }
            }

            let has_sp3 = summary
                .summaries
                .keys()
                .filter(|source| source.product_type == QcProductType::PreciseOrbit)
                .count()
                > 0;

            if has_sp3 {
                chapter.add_section(QcPdfSection::new("SP3"));
            }
        }

        chapters.push(chapter);

        Self { chapters }
    }
}
