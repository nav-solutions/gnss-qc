use std::collections::HashMap;

use crate::context::data::QcSourceDescriptor;

mod rinex_sum;
use rinex_sum::QcRINEXFileSummary;

use rinex::prelude::Header as RINEXHeader;

#[cfg(feature = "sp3")]
mod sp3_sum;

#[cfg(feature = "sp3")]
use sp3_sum::QcSP3FileSummary;

#[cfg(feature = "sp3")]
use sp3::prelude::Header as SP3Header;

/// [QcContextSummary] summary.
#[derive(Debug, Clone)]
pub enum QcFileSummary {
    RINEX(QcRINEXFileSummary),
    #[cfg(feature = "sp3")]
    SP3(QcSP3FileSummary),
}

/// [QcContextSummary] summary.
#[derive(Debug, Clone, Default)]
pub struct QcContextSummary {
    /// One small report per entry
    pub summaries: HashMap<QcSourceDescriptor, QcFileSummary>,
}

impl QcContextSummary {
    pub fn latch_rinex(&mut self, descriptor: QcSourceDescriptor, data: RINEXHeader) {
        self.summaries.insert(
            descriptor,
            QcFileSummary::RINEX(QcRINEXFileSummary::from_header(&data)),
        );
    }

    #[cfg(feature = "sp3")]
    pub fn latch_sp3(&mut self, descriptor: QcSourceDescriptor, data: SP3Header) {
        self.summaries.insert(
            descriptor,
            QcFileSummary::SP3(QcSP3FileSummary::from_header(&data)),
        );
    }
}
