use crate::report::QcRTKSummary;

use itertools::Itertools;

use crate::context::QcProductType;
use crate::report::summaries::{QcContextSummary, QcFileSummary};

use genpdf::{error::Error, render::Area, style::Style, Context, Element, Mm, RenderResult, Size};

use crate::report::rendering::pdf::{
    PDF_LARGE_VERTICAL_SPACING, PDF_MEDIUM_VERTICAL_SPACING, PDF_MIN_VERTICAL_SPACING,
};

impl QcRTKSummary {
    /// Render this [QcRTKSummary] as PDF content.
    pub fn render_pdf(&self) -> genpdf::elements::LinearLayout {
        let mut layout = genpdf::elements::LinearLayout::vertical();

        layout.push(
            genpdf::elements::Paragraph::new("RTK Summary")
                .styled(genpdf::style::Style::new().bold().with_font_size(10)),
        );

        let rovers = self.rovers.keys().collect::<Vec<_>>();
        let num_rovers = rovers.len();

        let bases = self.bases.keys().collect::<Vec<_>>();
        let num_bases = bases.len();

        let mut table = genpdf::elements::TableLayout::new(vec![1, 5]);

        table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

        if num_rovers == 0 {
            table
                .row()
                .element(
                    genpdf::elements::Paragraph::new("Rovers")
                        .styled(genpdf::style::Effect::Bold)
                        .padded(1),
                )
                .element(
                    genpdf::elements::Paragraph::new("None")
                        .styled(genpdf::style::Effect::Bold)
                        .padded(1),
                )
                .push()
                .expect("Invalid table row");
        } else {
            table
                .row()
                .element(
                    genpdf::elements::Paragraph::new("Rovers")
                        .styled(genpdf::style::Effect::Bold)
                        .padded(1),
                )
                .element(
                    genpdf::elements::Paragraph::new(rovers.iter().join(", "))
                        .styled(genpdf::style::Effect::Bold)
                        .padded(1),
                )
                .push()
                .expect("Invalid table row");
        }

        table
            .row()
            .element(
                genpdf::elements::Paragraph::new("Total")
                    .styled(genpdf::style::Effect::Bold)
                    .padded(1),
            )
            .element(
                genpdf::elements::Paragraph::new(format!("{}", num_rovers))
                    .styled(genpdf::style::Effect::Bold)
                    .padded(1),
            )
            .push()
            .expect("Invalid table row");

        table
            .row()
            .element(
                genpdf::elements::Paragraph::new("Bases")
                    .styled(genpdf::style::Effect::Bold)
                    .padded(1),
            )
            .element(
                genpdf::elements::Paragraph::new(bases.iter().join(", "))
                    .styled(genpdf::style::Effect::Bold)
                    .padded(1),
            )
            .push()
            .expect("Invalid table row");

        table
            .row()
            .element(
                genpdf::elements::Paragraph::new("Total")
                    .styled(genpdf::style::Effect::Bold)
                    .padded(1),
            )
            .element(
                genpdf::elements::Paragraph::new(format!("{}", num_bases))
                    .styled(genpdf::style::Effect::Bold)
                    .padded(1),
            )
            .push()
            .expect("Invalid table row");

        // for i in 0..10 {
        //     table
        //         .row()
        //         .element(genpdf::elements::Paragraph::new(format!("#{}", i)).padded(1))
        //         .element(genpdf::elements::Paragraph::new(LOREM_IPSUM).padded(1))
        //         .push()
        //         .expect("Invalid table row");
        // }

        layout.push(table);

        layout
    }
}
