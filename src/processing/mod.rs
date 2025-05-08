pub mod analysis;
mod runner;

use crate::{
    error::QcError,
    prelude::QcContext,
    processing::{analysis::QcAnalysisBuilder, runner::QcRunner},
    report::QcRunReport,
};

use hifitime::prelude::{Epoch, Unit};

impl QcContext {
    /// Process this [QcContext] running the following analysis specs.
    /// ## Input
    /// - current [QcContext]
    /// - [QcAnalysisBuilder] analysis specs
    /// ## Output
    /// - Synthesized [QcRunReport] that you can then render in your prefered format.
    pub fn process(&self, analysis: QcAnalysisBuilder) -> Result<QcRunReport, QcError> {
        let mut serializer = self.serializer();

        let deploy_time = Epoch::now()
            .map_err(|e| {
                error!("Failed to determine system time: {}", e);
                QcError::SystemTimeError
            })?
            .round(1.0 * Unit::Second);

        info!("process starting: {}", deploy_time);

        let mut report = QcRunReport::new(deploy_time, &analysis);

        let mut runner = QcRunner::new(&analysis, &mut report)?;

        // pull & consume data
        while let Some(sample) = serializer.next() {
            runner.consume(sample);
        }

        let end_time = Epoch::now()
            .unwrap_or_else(|e| {
                panic!("Failed to determine system time: {}", e);
            })
            .round(1.0 * Unit::Second);

        let run_duration = end_time - deploy_time;
        report.run_summary.run_duration = run_duration;

        info!("process concluded: {}", end_time);
        debug!("run duration: {}", run_duration);

        Ok(report)
    }
}

#[cfg(test)]
mod test {

    use std::fs::File;
    use std::io::Write;

    use crate::{prelude::QcContext, tests::init_logger};

    use super::QcAnalysisBuilder;
    #[test]
    fn process_light_full_run_no_sp3() {
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

        let html = report.render_html().into_string();
        let mut fd = File::create("index.html").unwrap();
        write!(fd, "{}", html).unwrap();
    }

    #[test]
    fn process_light_full_run() {
        init_logger();

        let mut ctx = QcContext::new();

        // load data
        ctx.load_rinex_file("data/OBS/V3/LARM0010.22O").unwrap();

        ctx.load_rinex_file("data/OBS/V3/LARM0630.22O").unwrap();

        ctx.load_rinex_file("data/OBS/V3/VLNS0010.22O").unwrap();

        ctx.load_rinex_file("data/OBS/V3/VLNS0630.22O").unwrap();

        ctx.load_gzip_rinex_file("data/MET/V3/POTS00DEU_R_20232540000_01D_05M_MM.rnx.gz")
            .unwrap();

        ctx.load_gzip_sp3_file("data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz")
            .unwrap();

        let builder = QcAnalysisBuilder::all();

        let report = ctx.process(builder).unwrap();

        let html = report.render_html().into_string();
        let mut fd = File::create("index.html").unwrap();
        write!(fd, "{}", html).unwrap();
    }

    #[test]
    fn process_full_run() {
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

        let html = report.render_html().into_string();
        let mut fd = File::create("index.html").unwrap();
        write!(fd, "{}", html).unwrap();
    }
}
