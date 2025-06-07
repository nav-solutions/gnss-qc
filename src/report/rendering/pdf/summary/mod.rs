use itertools::Itertools;

use crate::context::QcProductType;
use crate::report::summaries::{QcContextSummary, QcFileSummary};

use genpdf::{error::Error, render::Area, style::Style, Context, Element, Mm, RenderResult, Size};

use crate::report::rendering::pdf::{
    PDF_LARGE_VERTICAL_SPACING, PDF_MEDIUM_VERTICAL_SPACING, PDF_MIN_VERTICAL_SPACING,
};

impl QcContextSummary {
    /// Render this [QcContextSummary] as PDF content.
    pub fn render_pdf(&self) -> genpdf::elements::LinearLayout {

        let mut layout = genpdf::elements::LinearLayout::vertical();
        layout.push(genpdf::elements::Paragraph::new("Summary"));
    
        for (index, sum) in self.summaries.keys()
            .filter(|desc| desc.product_type.is_rinex_product())
            .sorted() 
            .enumerate()
        {

            if index == 0 {
                layout.push(genpdf::elements::Paragraph::new("RINEX"));
            }

            layout.push(genpdf::elements::Paragraph::new(&sum.filename));
        }
    
        for (index, sum) in self.summaries.keys()
            .filter(|desc| desc.product_type == QcProductType::PreciseOrbit) 
            .sorted() 
            .enumerate()
        {

            if index == 0 {
                layout.push(genpdf::elements::Paragraph::new("SP3"));
            }

            layout.push(genpdf::elements::Paragraph::new(&sum.filename));
        }

        layout
    }
    

}