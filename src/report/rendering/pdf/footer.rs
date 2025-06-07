use genpdf::Element;
use genpdf::render::Area;

#[derive(Default, Clone, Debug)]
pub struct QcPdfFooter {
    page: usize,
}

impl QcPdfFooter {

    pub fn new() -> Self {
        Self {
            page: 0,
        }
    }

    pub fn generate_footer(&self, top_padding: genpdf::Mm) -> Box<dyn genpdf::Element> {
        Box::new(
            genpdf::elements::Paragraph::new(format!("Page {}", self.page))
                .aligned(genpdf::Alignment::Right)
                .styled(genpdf::style::Style::new().with_font_size(8))
                .padded(genpdf::Margins::trbl(top_padding, 0, 0, 0)),
        )
    }
}

impl genpdf::PageDecorator for QcPdfFooter {
    fn decorate_page<'a>(
        &mut self,
        context: &genpdf::Context,
        mut area: genpdf::render::Area<'a>,
        style: genpdf::style::Style,
    ) -> Result<genpdf::render::Area<'a>, genpdf::error::Error> {
        self.page += 1;
        let mut footer_area = area.next_layer();

        //? Header
        let mut element = genpdf::elements::Paragraph::new("Header");
        area.add_margins(10);
        let result = element.render(context, area.clone(), style)?;
        area.add_offset(genpdf::Position::new(0, result.size.height + genpdf::Mm::from(5)));

        //? Footer
        footer_area.add_margins(genpdf::Margins::trbl(0, 10, 0, 10));
        let height = footer_area.size().height;
        let mut element = self.generate_footer(height - genpdf::Mm::from(11));
        let result = element.render(context, footer_area.clone(), style)?;
        let footer_size = result.size.height - height + genpdf::Mm::from(15);
        area.set_height(area.size().height - footer_size);

        Ok(area)
    }
}
