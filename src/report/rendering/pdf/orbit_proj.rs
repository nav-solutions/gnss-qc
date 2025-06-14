use crate::{
    prelude::{QcIndexing, Constellation},
    report::QcOrbitProjections,
};

impl QcOrbitProjections {
    /// Renders [QcOrbitProjections] as PDF element.
    pub fn render_pdf(&self, opts: &QcRenderingOptions) -> genpdf::elements::LinearLayout {

        let mut layout = genpdf::elements::LinearLayout::vertical();

        // TODO: adapt to PDF page dimensions
        let (width, height) = (2100, 2100);

        for (descriptor, constellation) in self
            .data
            .keys()
            .unique()
            .sorted()
        {
            if let Some(data) = self
                .data
                .get(&(descriptor.clone(), *constellation))
            {
                layout.push(genpdf::elements::PageBreak::new());
            }
        }
    }
}
