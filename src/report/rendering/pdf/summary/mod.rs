use genpdf::Element;
use itertools::Itertools;

use crate::context::{QcIndexing, QcProductType};
use crate::report::summaries::QcContextSummary;

impl QcContextSummary {
    /// Render this [QcContextSummary] as PDF content.
    pub fn render_pdf(&self) -> genpdf::elements::LinearLayout {
        let mut layout = genpdf::elements::LinearLayout::vertical();
        // layout.push(genpdf::elements::Paragraph::new("Summary"));

        for (index, sum) in self
            .summaries
            .keys()
            .filter(|desc| desc.product_type.is_rinex_product())
            .sorted()
            .enumerate()
        {
            if index > 0 {
                layout.push(genpdf::elements::PageBreak::new());
            }

            let mut table = genpdf::elements::TableLayout::new(vec![1, 5]);

            table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

            table
                .row()
                .element(
                    genpdf::elements::Paragraph::new("Filename")
                        .styled(genpdf::style::Effect::Bold)
                        .padded(1),
                )
                .element(
                    genpdf::elements::Paragraph::new(&sum.filename)
                        .styled(genpdf::style::Effect::Bold)
                        .padded(1),
                )
                .push()
                .expect("Invalid table row");

            table
                .row()
                .element(
                    genpdf::elements::Paragraph::new("Format")
                        .styled(genpdf::style::Effect::Bold)
                        .padded(1),
                )
                .element(
                    genpdf::elements::Paragraph::new(sum.product_type.to_string())
                        .styled(genpdf::style::Effect::Bold)
                        .padded(1),
                )
                .push()
                .expect("Invalid table row");

            if let Some(summary) = self.summaries.get(&sum) {
                let summary = summary
                    .as_rinex()
                    .expect("internal error: rinex data access");

                table
                    .row()
                    .element(
                        genpdf::elements::Paragraph::new("Version")
                            .styled(genpdf::style::Effect::Bold)
                            .padded(1),
                    )
                    .element(
                        genpdf::elements::Paragraph::new(summary.version.to_string()).padded(1),
                    )
                    .push()
                    .expect("Invalid table row");
            }

            let indexed_by = match &sum.indexing {
                QcIndexing::Agency(agency) => "Agency",
                QcIndexing::Custom(custom) => "Custom",
                QcIndexing::GeodeticMarker(marker) => "Marker",
                QcIndexing::GnssReceiver(gnss) => "GNSS Receiver",
                QcIndexing::None => "None",
                QcIndexing::Operator(operator) => "Operator",
                QcIndexing::RxAntenna(ant) => "GNSS Antenna",
            };

            table
                .row()
                .element(
                    genpdf::elements::Paragraph::new("Indexing")
                        .styled(genpdf::style::Effect::Bold)
                        .padded(1),
                )
                .element(genpdf::elements::Paragraph::new(indexed_by).padded(1))
                .push()
                .expect("Invalid table row");

            table
                .row()
                .element(
                    genpdf::elements::Paragraph::new("Index")
                        .styled(genpdf::style::Effect::Bold)
                        .padded(1),
                )
                .element(genpdf::elements::Paragraph::new(sum.indexing.to_string()).padded(1))
                .push()
                .expect("Invalid table row");

            if let Some(summary) = self.summaries.get(&sum) {
                let summary = summary
                    .as_rinex()
                    .expect("internal error: rinex data access");

                if let Some(timescale) = &summary.timescale {
                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("Timescale")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(genpdf::elements::Paragraph::new(timescale.to_string()).padded(1))
                        .push()
                        .expect("Invalid table row");
                } else {
                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("Timescale")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(genpdf::elements::Paragraph::new("N.a").padded(1))
                        .push()
                        .expect("Invalid table row");
                }

                if let Some(receiver) = &summary.receiver {
                    let formatted = format!("{}{}", receiver.model, receiver.sn);

                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("GNSS-Receiver")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(genpdf::elements::Paragraph::new(formatted).padded(1))
                        .push()
                        .expect("Invalid table row");
                }

                if let Some(antenna) = &summary.antenna {
                    let formatted = format!("{}{}", antenna.model, antenna.sn);

                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("GNSS-Antenna")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(genpdf::elements::Paragraph::new(formatted).padded(1))
                        .push()
                        .expect("Invalid table row");
                }

                if let Some(marker) = &summary.geodetic_marker {
                    let formatted = if let Some(number) = marker.number() {
                        format!("{}-{}", &marker.name, number)
                    } else {
                        marker.name.clone()
                    };

                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("Marker")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(genpdf::elements::Paragraph::new(formatted).padded(1))
                        .push()
                        .expect("Invalid table row");
                }

                layout.push(table);

                // if let Some(position) = &summary.reference_position {
                //     let wgs84 = position.to_earth_geodetic_degrees_km();

                //     table
                //         .row()
                //         .element(
                //             genpdf::elements::Paragraph::new("Reference Position (WGS84)")
                //                 .styled(genpdf::style::Effect::Bold)
                //                 .padded(1),
                //         )
                //         .push()
                //         .expect("Invalid table row");
                // }

                if !summary.v3_time_corrections.is_empty() {
                    layout.push(genpdf::elements::Break::new(1.0));
                    layout.push(
                        genpdf::elements::Paragraph::new("Time Corrections")
                            .styled(genpdf::style::Effect::Bold)
                            .padded(1),
                    );

                    let mut table = genpdf::elements::TableLayout::new(vec![1, 5]);

                    table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(
                        true, true, false,
                    ));

                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("Target")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(
                            genpdf::elements::Paragraph::new("Reference")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .push()
                        .expect("Invalid table row");

                    for (target, reference) in summary.v3_time_corrections.iter().sorted() {
                        table
                            .row()
                            .element(genpdf::elements::Paragraph::new(target.to_string()).padded(1))
                            .element(
                                genpdf::elements::Paragraph::new(reference.to_string()).padded(1),
                            )
                            .push()
                            .expect("Invalid table row");
                    }

                    layout.push(table);
                }
            }
        }

        for (index, sum) in self
            .summaries
            .keys()
            .filter(|desc| desc.product_type == QcProductType::PreciseOrbit)
            .sorted()
            .enumerate()
        {
            layout.push(genpdf::elements::PageBreak::new());

            let mut table = genpdf::elements::TableLayout::new(vec![1, 5]);

            table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

            table
                .row()
                .element(
                    genpdf::elements::Paragraph::new("Filename")
                        .styled(genpdf::style::Effect::Bold)
                        .padded(1),
                )
                .element(
                    genpdf::elements::Paragraph::new(sum.filename.to_string())
                        .styled(genpdf::style::Effect::Bold)
                        .padded(1),
                )
                .push()
                .expect("Invalid table row");

            table
                .row()
                .element(
                    genpdf::elements::Paragraph::new("Indexed by")
                        .styled(genpdf::style::Effect::Bold)
                        .padded(1),
                )
                .element(genpdf::elements::Paragraph::new("Agency").padded(1))
                .push()
                .expect("Invalid table row");

            if let Some(sum) = self.summaries.get(&sum) {
                let summary = sum.as_sp3().expect("internal error: sp3 data");

                table
                    .row()
                    .element(
                        genpdf::elements::Paragraph::new("Version")
                            .styled(genpdf::style::Effect::Bold)
                            .padded(1),
                    )
                    .element(
                        genpdf::elements::Paragraph::new(summary.version.to_string()).padded(1),
                    )
                    .push()
                    .expect("Invalid table row");

                table
                    .row()
                    .element(
                        genpdf::elements::Paragraph::new("Agency")
                            .styled(genpdf::style::Effect::Bold)
                            .padded(1),
                    )
                    .element(genpdf::elements::Paragraph::new(&summary.agency).padded(1))
                    .push()
                    .expect("Invalid table row");

                table
                    .row()
                    .element(
                        genpdf::elements::Paragraph::new("Timescale")
                            .styled(genpdf::style::Effect::Bold)
                            .padded(1),
                    )
                    .element(
                        genpdf::elements::Paragraph::new(summary.timescale.to_string()).padded(1),
                    )
                    .push()
                    .expect("Invalid table row");

                table
                    .row()
                    .element(
                        genpdf::elements::Paragraph::new("Orbit Type")
                            .styled(genpdf::style::Effect::Bold)
                            .padded(1),
                    )
                    .element(genpdf::elements::Paragraph::new(&summary.orbit_type).padded(1))
                    .push()
                    .expect("Invalid table row");

                table
                    .row()
                    .element(
                        genpdf::elements::Paragraph::new("ECEF")
                            .styled(genpdf::style::Effect::Bold)
                            .padded(1),
                    )
                    .element(genpdf::elements::Paragraph::new(&summary.frame).padded(1))
                    .push()
                    .expect("Invalid table row");

                if summary.has_sv_velocities {
                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("Velocities")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(genpdf::elements::Paragraph::new("True").padded(1))
                        .push()
                        .expect("Invalid table row");
                } else {
                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("Velocities")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(genpdf::elements::Paragraph::new("False").padded(1))
                        .push()
                        .expect("Invalid table row");
                }

                if summary.has_sv_clock_offsets {
                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("Clock Offset")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(genpdf::elements::Paragraph::new("True").padded(1))
                        .push()
                        .expect("Invalid table row");
                } else {
                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("Clock Offset")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(genpdf::elements::Paragraph::new("False").padded(1))
                        .push()
                        .expect("Invalid table row");
                }

                if summary.has_sv_clock_drift {
                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("Clock Drift")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(genpdf::elements::Paragraph::new("True").padded(1))
                        .push()
                        .expect("Invalid table row");
                } else {
                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("Clock Drift")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(genpdf::elements::Paragraph::new("False").padded(1))
                        .push()
                        .expect("Invalid table row");
                }

                if summary.has_sv_clock_event {
                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("Clock Events (bumps)")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(genpdf::elements::Paragraph::new("Yes").padded(1))
                        .push()
                        .expect("Invalid table row");
                } else {
                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("Clock Events (bumps)")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(genpdf::elements::Paragraph::new("None").padded(1))
                        .push()
                        .expect("Invalid table row");
                }

                if summary.has_sv_maneuver {
                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("Satellite maneuvers")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(genpdf::elements::Paragraph::new("Yes").padded(1))
                        .push()
                        .expect("Invalid table row");
                } else {
                    table
                        .row()
                        .element(
                            genpdf::elements::Paragraph::new("Satellite maneuvers")
                                .styled(genpdf::style::Effect::Bold)
                                .padded(1),
                        )
                        .element(genpdf::elements::Paragraph::new("None").padded(1))
                        .push()
                        .expect("Invalid table row");
                }
            }

            layout.push(table)
        }

        layout
    }
}
