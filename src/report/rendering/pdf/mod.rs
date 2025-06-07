//! HTML rendition of the QcRunReport
use crate::report::QcRunReport;

mod summary;

mod font;
use font::QcPdfFontFamily;

mod title;
use title::QcPdfTitle;

mod subtitle;
use subtitle::QcPdfSubtitle;

pub mod vertical_separator;

mod table_of_content;
use table_of_content::QcPdfTableOfContent;

mod documentation;
use documentation::QcPdfDocumentation;

mod credits;
use credits::QcPdfCredits;

use genpdf::Element;

pub(crate) const PDF_LARGE_VERTICAL_SPACING: f64 = 3.0;
pub(crate) const PDF_MIN_VERTICAL_SPACING: f64 = 0.3;
pub(crate) const PDF_MEDIUM_VERTICAL_SPACING: f64 = 1.5;

impl QcRunReport {
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
        // doc.push(genpdf::elements::PageBreak::new());
        doc.push(QcPdfTableOfContent::new(&self));

        // documentation
        // doc.push(genpdf::elements::PageBreak::new());
        doc.push(QcPdfDocumentation::new());

        // credits
        // doc.push(genpdf::elements::PageBreak::new());
        doc.push(QcPdfCredits::new());

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

    #[test]
    fn pdf_full_run_24h() {
        init_logger();

        let mut ctx = QcContext::new();

        // load data
        ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
            .unwrap();

        ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
            .unwrap();

        ctx.load_gzip_rinex_file("data/CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz")
            .unwrap();

        ctx.load_gzip_sp3_file("data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz")
            .unwrap();

        let builder = QcAnalysisBuilder::all();

        let report = ctx.process(builder).unwrap();

        report.render_pdf().render_to_file("report.pdf").unwrap();
    }

    #[test]
    fn pdf_jmf_longterm() {
        init_logger();

        let mut ctx = QcContext::new();

        // load data
        ctx.load_rinex_file("data/DataJMF/2024-09-18_00-00-00_GNSS-1.24o")
            .unwrap();

        ctx.load_rinex_file("data/DataJMF/2024-09-19_00-00-00_GNSS-1.obs")
            .unwrap();

        ctx.load_rinex_file("data/DataJMF/2025-04-29_19-53-50_GNSS-1.obs")
            .unwrap();

        ctx.load_rinex_file("data/DataJMF/240428survey.obs")
            .unwrap();

        let builder = QcAnalysisBuilder::all();

        let report = ctx.process(builder).unwrap();

        report.render_pdf().render_to_file("report.pdf").unwrap();
    }
}
