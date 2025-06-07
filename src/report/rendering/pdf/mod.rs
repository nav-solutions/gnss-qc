//! HTML rendition of the QcRunReport
use crate::report::QcRunReport;
use latex::Document;

mod summary;

mod font;
use font::QcPdfFontFamily;

mod title;
use title::QcPdfTitle;

mod subtitle;
use subtitle::QcPdfSubtitle;

mod table_of_content;
use table_of_content::QcPdfTableOfContent;

use genpdf::Element;

pub(crate) const PDF_LARGE_VERTICAL_SPACING: f64 = 3.0;
pub(crate) const PDF_MIN_VERTICAL_SPACING: f64 = 0.3;
pub(crate) const PDF_MEDIUM_VERTICAL_SPACING: f64 = 1.5;

impl QcRunReport {
    /// Render this [QcRunReport] as a `LateX` [Document].
    pub fn render_latex(&self) -> Document {
        let mut doc = Document::new(latex::DocumentClass::Article);
        doc.preamble.title("GNSS-QC Report");
        doc.preamble.author(&format!(
            "RTK-rs framework <https://github.com/rtk-rs> v{}",
            env!("CARGO_PKG_VERSION")
        ));

        // link rel="icon" type="image/x-icon" href="https://raw.githubusercontent.com/rtk-rs/.github/master/logos/logo2.jpg";

        // run report
        // summary
        // RTK Summary

        doc
    }

    /// Render this [QcRunReport] as PDF [genpdf::Document].
    pub fn render_pdf(&self) -> genpdf::Document {
        let font = QcPdfFontFamily::new();
        let mut doc = genpdf::Document::new(font);

        doc.set_title("GNSS-QC Report");
        doc.set_minimal_conformance();
        doc.set_line_spacing(1.25);

        let mut decorator = genpdf::SimplePageDecorator::new();
        decorator.set_margins(10);

        decorator.set_header(|page| {
            let mut layout = genpdf::elements::LinearLayout::vertical();
            if page > 1 {
                layout.push(
                    genpdf::elements::Paragraph::new(format!("Page {}", page))
                        .aligned(genpdf::Alignment::Center),
                );
                layout.push(genpdf::elements::Break::new(1));
            }
            layout.styled(genpdf::style::Style::new().with_font_size(10))
        });

        doc.set_page_decorator(decorator);

        // title
        doc.push(QcPdfTitle::new());
        doc.push(genpdf::elements::Break::new(PDF_MIN_VERTICAL_SPACING));
        doc.push(QcPdfSubtitle::new());

        // table of content
        doc.push(QcPdfTableOfContent::new(&self));

        doc
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::QcAnalysisBuilder;
    /**
     * Test PDF rendition using meaningful setups
     */
    use crate::{prelude::QcContext, tests::init_logger};

    #[test]
    fn pdf_no_sp3() {
        init_logger();

        let mut ctx = QcContext::new();

        // load data
        ctx.load_rinex_file("data/OBS/V3/LARM0010.22O").unwrap();

        ctx.load_rinex_file("data/OBS/V3/LARM0630.22O").unwrap();

        ctx.load_rinex_file("data/OBS/V3/VLNS0010.22O").unwrap();

        ctx.load_rinex_file("data/OBS/V3/VLNS0630.22O").unwrap();

        ctx.load_gzip_rinex_file("data/MET/V3/POTS00DEU_R_20232540000_01D_05M_MM.rnx.gz")
            .unwrap();

        let builder = QcAnalysisBuilder::all();

        let report = ctx.process(builder).unwrap();

        report.render_pdf().render_to_file("report.pdf").unwrap();
    }
}
