//! Regroups all known Analysis and runner
mod runner;

pub mod analysis;

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

        let mut runner = QcRunner::new(&analysis, &mut report, self.earth_cef)?;

        // consume all data
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
