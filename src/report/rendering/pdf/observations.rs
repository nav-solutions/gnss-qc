use crate::report::QcObservationsReport;
use genpdf::Element;

use itertools::Itertools;
use plotters::style::WHITE;

use log::error;

use plotters::prelude::*;

// use plotters::coord::ranged1d::AsRangedCoord;
// use plotters::coord::ranged1d::Ranged;

use plotters::style::Color;

use image::DynamicImage;
use image::ImageBuffer;

use crate::prelude::Epoch;

impl QcObservationsReport {
    /// Renders this [QcObservationsReport] to PDF content.
    pub fn render_pdf(&self) -> genpdf::elements::LinearLayout {
        let mut layout = genpdf::elements::LinearLayout::vertical();

        for descriptor in self.data.keys().map(|(key, _)| key).unique().sorted() {
            for (nth_constell, constellation) in self
                .data
                .keys()
                .filter_map(|(key, constell)| {
                    if key == descriptor {
                        Some(constell)
                    } else {
                        None
                    }
                })
                .sorted()
                .enumerate()
            {
                if let Some(data) = self.data.get(&(descriptor.clone(), *constellation)) {
                    layout.push(genpdf::elements::PageBreak::new());

                    let name =
                        format!("{} {} Pseudo Range Observations", descriptor, constellation);

                    // layout.push(
                    //     genpdf::elements::Paragraph::new(name)
                    //         .styled(genpdf::style::Style::new().bold().with_font_size(10)),
                    // );

                    let (width, height) = (2000, 2000);

                    let mut buffer = ImageBuffer::new(width, height);

                    {
                        let mut backend = BitMapBackend::with_buffer(&mut buffer, (width, height))
                            .into_drawing_area();

                        backend.fill(&WHITE).unwrap();

                        // building x, y axis
                        let mut x_spec = f64::INFINITY..-f64::INFINITY;
                        let mut y_spec = f64::INFINITY..-f64::INFINITY;

                        for (nth_data, (_, data)) in data.pseudo_range_m.iter().enumerate() {
                            let (xmin, xmax) = (data.xmin(), data.xmax());
                            let (ymin, ymax) = (data.ymin(), data.ymax());

                            if nth_data == 0 || xmin < x_spec.start {
                                x_spec.start = xmin;
                            }

                            if nth_data == 0 || xmax > x_spec.end {
                                x_spec.end = xmax;
                            }

                            if nth_data == 0 || ymin < y_spec.start {
                                y_spec.start = ymin;
                            }

                            if nth_data == 0 || ymax > y_spec.end {
                                y_spec.end = ymax;
                            }
                        }

                        // build chart
                        let mut chart = ChartBuilder::on(&backend)
                            .margin(10)
                            .set_left_and_bottom_label_area_size(20)
                            .caption(name, ("sans-serif", 30))
                            .x_label_area_size(30)
                            .y_label_area_size(30)
                            .build_cartesian_2d(x_spec, y_spec)
                            .unwrap();

                        chart.configure_mesh().draw().unwrap();

                        for ((sv, carrier), data) in &data.pseudo_range_m {
                            let curve_title = format!("{}({})", sv, carrier);
                            data.draw(&mut chart, &curve_title, RED, 4);
                        }

                        // backend.present().unwrap();
                    }

                    let image = DynamicImage::ImageRgb8(buffer);

                    match genpdf::elements::Image::from_dynamic_image(image) {
                        Ok(image) => {
                            layout.push(image);
                        }
                        Err(e) => {
                            error!("Drawing error: {}", e);
                            let formatted = format!("Drawing error: {}", e);

                            layout.push(
                                genpdf::elements::Paragraph::new(formatted)
                                    .styled(genpdf::style::Style::new().bold().with_font_size(10)),
                            );
                        }
                    }
                }
            }
        }

        layout
    }
}
