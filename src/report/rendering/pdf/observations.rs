use crate::report::QcObservationsReport;
use genpdf::Element;

use itertools::Itertools;

use log::error;

use plotters::{
    drawing::IntoDrawingArea,
    prelude::{BitMapBackend, ChartBuilder, ColorMap, SeriesLabelPosition, VulcanoHSL},
    style::{Color, BLACK, WHITE},
};

use image::{DynamicImage, ImageBuffer};

impl QcObservationsReport {
    /// Renders this [QcObservationsReport] to PDF content.
    pub fn render_pdf(&self) -> genpdf::elements::LinearLayout {
        let mut layout = genpdf::elements::LinearLayout::vertical();

        let cmap = VulcanoHSL;

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

                    let mut sv_uniques = Vec::new();
                    let (width, height) = (2100, 2100);

                    let mut buffer = ImageBuffer::new(width, height);

                    let chart_name =
                        format!("{} {} Pseudo Range Observations", descriptor, constellation);
                    {
                        let mut backend = BitMapBackend::with_buffer(&mut buffer, (width, height))
                            .into_drawing_area();

                        backend.fill(&WHITE).unwrap();

                        // x, y axis
                        let (mut x_spec, mut y_spec) =
                            (f64::INFINITY..-f64::INFINITY, f64::INFINITY..-f64::INFINITY);

                        for (nth_data, ((sv, carrier), data)) in
                            data.pseudo_range_m.iter().enumerate()
                        {
                            let (xmin, xmax) = (data.xmin(), data.xmax());
                            let (ymin, ymax) = (data.ymin(), data.ymax());

                            if nth_data == 0 || xmin < x_spec.start {
                                x_spec.start = xmin;
                            }

                            if nth_data == 0 || ymin < y_spec.start {
                                y_spec.start = ymin;
                            }

                            if nth_data == 0 || xmax > x_spec.end {
                                x_spec.end = xmax;
                            }

                            if nth_data == 0 || ymax > y_spec.end {
                                y_spec.end = ymax;
                            }

                            if !sv_uniques.contains(&sv) {
                                sv_uniques.push(sv);
                            }
                        }

                        y_spec.start = 0.95 * y_spec.start;
                        y_spec.end = 1.05 * y_spec.end;

                        let cmap_len = sv_uniques.len() as f64;

                        // build chart
                        let mut chart = ChartBuilder::on(&backend)
                            .margin(15)
                            .set_left_and_bottom_label_area_size(100)
                            .caption(chart_name, ("sans-serif", 100))
                            .x_label_area_size(100)
                            .y_label_area_size(100)
                            .build_cartesian_2d(x_spec, y_spec)
                            .unwrap();

                        chart.configure_mesh().draw().unwrap();

                        for (nth_sv, sv) in data
                            .pseudo_range_m
                            .keys()
                            .map(|(sv, _)| sv)
                            .unique()
                            .enumerate()
                        {
                            for carrier in
                                data.pseudo_range_m.keys().filter_map(|(sv_i, carrier)| {
                                    if sv_i == sv {
                                        Some(carrier)
                                    } else {
                                        None
                                    }
                                })
                            {
                                let sv_color = nth_sv as f64 / cmap_len;
                                let sv_color = cmap.get_color(sv_color);

                                if let Some(data) = data.pseudo_range_m.get(&(*sv, *carrier)) {
                                    let curve_title = format!("{}({})", sv, carrier);
                                    data.draw(&mut chart, &curve_title, sv_color, 4);
                                }
                            }
                        }

                        chart
                            .configure_series_labels()
                            .label_font(("sans-serif", 30))
                            .border_style(&BLACK)
                            .background_style(&WHITE.mix(0.8))
                            .position(SeriesLabelPosition::UpperRight)
                            .draw()
                            .unwrap();
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

                    let chart_name =
                        format!("{} {} Phase Range Observations", descriptor, constellation);

                    let mut buffer = ImageBuffer::new(width, height);

                    {
                        let mut backend = BitMapBackend::with_buffer(&mut buffer, (width, height))
                            .into_drawing_area();

                        backend.fill(&WHITE).unwrap();

                        // x, y axis
                        let (mut x_spec, mut y_spec) =
                            (f64::INFINITY..-f64::INFINITY, f64::INFINITY..-f64::INFINITY);

                        for (nth_data, ((sv, carrier), data)) in
                            data.phase_range_m.iter().enumerate()
                        {
                            let (xmin, xmax) = (data.xmin(), data.xmax());
                            let (ymin, ymax) = (data.ymin(), data.ymax());

                            if nth_data == 0 || xmin < x_spec.start {
                                x_spec.start = xmin;
                            }

                            if nth_data == 0 || ymin < y_spec.start {
                                y_spec.start = ymin;
                            }

                            if nth_data == 0 || xmax > x_spec.end {
                                x_spec.end = xmax;
                            }

                            if nth_data == 0 || ymax > y_spec.end {
                                y_spec.end = ymax;
                            }

                            if !sv_uniques.contains(&sv) {
                                sv_uniques.push(sv);
                            }
                        }

                        y_spec.start = 0.95 * y_spec.start;
                        y_spec.end = 1.05 * y_spec.end;

                        let cmap_len = sv_uniques.len() as f64;

                        // build chart
                        let mut chart = ChartBuilder::on(&backend)
                            .margin(15)
                            .set_left_and_bottom_label_area_size(100)
                            .caption(chart_name, ("sans-serif", 100))
                            .x_label_area_size(100)
                            .y_label_area_size(100)
                            .build_cartesian_2d(x_spec, y_spec)
                            .unwrap();

                        chart.configure_mesh().draw().unwrap();

                        for (nth_sv, sv) in data
                            .phase_range_m
                            .keys()
                            .map(|(sv, _)| sv)
                            .unique()
                            .enumerate()
                        {
                            for carrier in
                                data.phase_range_m.keys().filter_map(|(sv_i, carrier)| {
                                    if sv_i == sv {
                                        Some(carrier)
                                    } else {
                                        None
                                    }
                                })
                            {
                                let sv_color = nth_sv as f64 / cmap_len;
                                let sv_color = cmap.get_color(sv_color);

                                if let Some(data) = data.phase_range_m.get(&(*sv, *carrier)) {
                                    let curve_title = format!("{}({})", sv, carrier);
                                    data.draw(&mut chart, &curve_title, sv_color, 4);
                                }
                            }
                        }

                        chart
                            .configure_series_labels()
                            .label_font(("sans-serif", 30))
                            .border_style(&BLACK)
                            .background_style(&WHITE.mix(0.8))
                            .position(SeriesLabelPosition::UpperRight)
                            .draw()
                            .unwrap();
                    }

                    let image = DynamicImage::ImageRgb8(buffer);

                    match genpdf::elements::Image::from_dynamic_image(image) {
                        Ok(image) => {
                            layout.push(genpdf::elements::PageBreak::new());
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

                    let chart_name =
                        format!("{} {} Doppler Observations", descriptor, constellation);

                    let mut buffer = ImageBuffer::new(width, height);

                    {
                        let mut backend = BitMapBackend::with_buffer(&mut buffer, (width, height))
                            .into_drawing_area();

                        backend.fill(&WHITE).unwrap();

                        // x, y axis
                        let (mut x_spec, mut y_spec) =
                            (f64::INFINITY..-f64::INFINITY, f64::INFINITY..-f64::INFINITY);

                        for (nth_data, ((sv, carrier), data)) in
                            data.phase_range_m.iter().enumerate()
                        {
                            let (xmin, xmax) = (data.xmin(), data.xmax());
                            let (ymin, ymax) = (data.ymin(), data.ymax());

                            if nth_data == 0 || xmin < x_spec.start {
                                x_spec.start = xmin;
                            }

                            if nth_data == 0 || ymin < y_spec.start {
                                y_spec.start = ymin;
                            }

                            if nth_data == 0 || xmax > x_spec.end {
                                x_spec.end = xmax;
                            }

                            if nth_data == 0 || ymax > y_spec.end {
                                y_spec.end = ymax;
                            }

                            if !sv_uniques.contains(&sv) {
                                sv_uniques.push(sv);
                            }
                        }

                        y_spec.start = 0.95 * y_spec.start;
                        y_spec.end = 1.05 * y_spec.end;

                        let cmap_len = sv_uniques.len() as f64;

                        // build chart
                        let mut chart = ChartBuilder::on(&backend)
                            .margin(15)
                            .set_left_and_bottom_label_area_size(100)
                            .caption(chart_name, ("sans-serif", 100))
                            .x_label_area_size(100)
                            .y_label_area_size(100)
                            .build_cartesian_2d(x_spec, y_spec)
                            .unwrap();

                        chart.configure_mesh().draw().unwrap();

                        for (nth_sv, sv) in
                            data.doppler.keys().map(|(sv, _)| sv).unique().enumerate()
                        {
                            for carrier in data.doppler.keys().filter_map(|(sv_i, carrier)| {
                                if sv_i == sv {
                                    Some(carrier)
                                } else {
                                    None
                                }
                            }) {
                                let sv_color = nth_sv as f64 / cmap_len;
                                let sv_color = cmap.get_color(sv_color);

                                if let Some(data) = data.doppler.get(&(*sv, *carrier)) {
                                    let curve_title = format!("{}({})", sv, carrier);
                                    data.draw(&mut chart, &curve_title, sv_color, 4);
                                }
                            }
                        }

                        chart
                            .configure_series_labels()
                            .label_font(("sans-serif", 30))
                            .border_style(&BLACK)
                            .background_style(&WHITE.mix(0.8))
                            .position(SeriesLabelPosition::UpperRight)
                            .draw()
                            .unwrap();
                    }

                    let image = DynamicImage::ImageRgb8(buffer);

                    match genpdf::elements::Image::from_dynamic_image(image) {
                        Ok(image) => {
                            layout.push(genpdf::elements::PageBreak::new());
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
