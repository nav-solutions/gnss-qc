use crate::report::QcObservationsReport;
use genpdf::Element;

use itertools::Itertools;
use plotters::style::WHITE;

use log::error;

impl QcObservationsReport {
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

                    layout.push(
                        genpdf::elements::Paragraph::new(name)
                            .styled(genpdf::style::Style::new().bold().with_font_size(10)),
                    );

                    for ((sv, carrier), data) in &data.pseudo_range_m {
                        let plot = data.to_cartesian_2d("test", 100, 100, WHITE);

                        match genpdf::elements::Image::from_dynamic_image(plot) {
                            Ok(image) => {
                                layout.push(image);
                            }
                            Err(e) => {
                                error!("Drawing error: {}", e);
                                let formatted = format!("Drawing error: {}", e);

                                layout.push(
                                    genpdf::elements::Paragraph::new(formatted).styled(
                                        genpdf::style::Style::new().bold().with_font_size(10),
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }

        layout
    }
}
