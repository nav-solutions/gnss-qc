//! HTML rendition of the QcRunReport
use crate::report::QcRunReport;
use latex::Document;

mod summary;

use genpdf::fonts::FontFamily;

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
        let font_family =
            genpdf::fonts::from_files("fonts", "DejaVuSans", None).unwrap_or_else(|e| {
                panic!("Failed to load fonts: {}", e);
            });

        let mut doc = genpdf::Document::new(font_family);

        doc
    }
}

#[cfg(test)]
mod test {
    /**
     * Test PDF rendition using meaningful setups
     */
    use std::fs::File;
    use std::io::Write;

    use crate::{prelude::QcContext, tests::init_logger};

    use crate::prelude::QcAnalysisBuilder;

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
