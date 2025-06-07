use crate::report::QcRTKSummary;

use itertools::Itertools;

use genpdf::Element;

impl QcRTKSummary {
    /// Render this [QcRTKSummary] as PDF content.
    pub fn render_pdf(&self) -> genpdf::elements::LinearLayout {
        let mut layout = genpdf::elements::LinearLayout::vertical();

        // layout.push(
        //     genpdf::elements::Paragraph::new("RTK Summary")
        //         .styled(genpdf::style::Style::new().bold().with_font_size(10)),
        // );

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
                .element(genpdf::elements::Paragraph::new("None").padded(1))
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
                .element(genpdf::elements::Paragraph::new(rovers.iter().join(", ")).padded(1))
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
            .element(genpdf::elements::Paragraph::new(format!("{}", num_rovers)).padded(1))
            .push()
            .expect("Invalid table row");

        table
            .row()
            .element(
                genpdf::elements::Paragraph::new("Bases")
                    .styled(genpdf::style::Effect::Bold)
                    .padded(1),
            )
            .element(genpdf::elements::Paragraph::new(bases.iter().join(", ")).padded(1))
            .push()
            .expect("Invalid table row");

        table
            .row()
            .element(
                genpdf::elements::Paragraph::new("Total")
                    .styled(genpdf::style::Effect::Bold)
                    .padded(1),
            )
            .element(genpdf::elements::Paragraph::new(format!("{}", num_bases)).padded(1))
            .push()
            .expect("Invalid table row");

        layout.push(table);

        layout.push(genpdf::elements::Break::new(1.0));
        layout.push(
            genpdf::elements::Paragraph::new("Base network")
                .styled(genpdf::style::Style::new().bold().with_font_size(10)),
        );

        let mut table = genpdf::elements::TableLayout::new(vec![5, 5, 5]);
        table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

        table
            .row()
            .element(
                genpdf::elements::Paragraph::new("Base")
                    .styled(genpdf::style::Effect::Bold)
                    .padded(1),
            )
            .element(
                genpdf::elements::Paragraph::new("Base")
                    .styled(genpdf::style::Effect::Bold)
                    .padded(1),
            )
            .element(
                genpdf::elements::Paragraph::new("Baseline (km)")
                    .styled(genpdf::style::Effect::Bold)
                    .padded(1),
            )
            .push()
            .expect("Invalid table row");

        for ((base1, base2), dist_km) in self.base_network_distances_km.iter() {
            table
                .row()
                .element(genpdf::elements::Paragraph::new(base1).padded(1))
                .element(genpdf::elements::Paragraph::new(base2).padded(1))
                .element(genpdf::elements::Paragraph::new(format!("{:3.3}", dist_km)).padded(1))
                .push()
                .expect("Invalid table row");
        }

        layout.push(table);

        layout.push(genpdf::elements::Break::new(1.0));
        layout.push(
            genpdf::elements::Paragraph::new("Baselines")
                .styled(genpdf::style::Style::new().bold().with_font_size(10)),
        );

        let mut table = genpdf::elements::TableLayout::new(vec![5, 5, 5]);
        table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

        table
            .row()
            .element(
                genpdf::elements::Paragraph::new("Rover")
                    .styled(genpdf::style::Effect::Bold)
                    .padded(1),
            )
            .element(
                genpdf::elements::Paragraph::new("Base")
                    .styled(genpdf::style::Effect::Bold)
                    .padded(1),
            )
            .element(
                genpdf::elements::Paragraph::new("Baseline (km)")
                    .styled(genpdf::style::Effect::Bold)
                    .padded(1),
            )
            .push()
            .expect("Invalid table row");

        for ((rover, base), dist_km) in self.baseline_distances_km.iter() {
            table
                .row()
                .element(genpdf::elements::Paragraph::new(rover).padded(1))
                .element(genpdf::elements::Paragraph::new(base).padded(1))
                .element(genpdf::elements::Paragraph::new(format!("{:3.3}", dist_km)).padded(1))
                .push()
                .expect("Invalid table row");
        }

        layout.push(table);

        layout
    }
}
