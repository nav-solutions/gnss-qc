use std::collections::HashMap;

use crate::context::data::QcSourceDescriptor;

pub(crate) mod rinex_sum;
use rinex_sum::QcRINEXFileSummary;

use rinex::prelude::Header as RINEXHeader;

#[cfg(feature = "sp3")]
pub(crate) mod sp3_sum;

#[cfg(feature = "sp3")]
use sp3_sum::QcSP3FileSummary;

#[cfg(feature = "sp3")]
use sp3::prelude::Header as SP3Header;

#[cfg(feature = "navigation")]
use crate::prelude::Frame;

/// [QcContextSummary] summary.
#[derive(Debug, Clone)]
pub enum QcFileSummary {
    /// [QcRINEXFileSummary]
    RINEX(QcRINEXFileSummary),

    /// [QcSP3FileSummary]
    #[cfg(feature = "sp3")]
    SP3(QcSP3FileSummary),
}

impl QcFileSummary {
    pub fn as_rinex(&self) -> Option<&QcRINEXFileSummary> {
        match self {
            Self::RINEX(sum) => Some(sum),
            _ => None,
        }
    }

    pub fn as_sp3(&self) -> Option<&QcSP3FileSummary> {
        match self {
            Self::SP3(sum) => Some(sum),
            _ => None,
        }
    }
}

/// [QcContextSummary] summary.
#[derive(Debug, Clone)]
pub struct QcContextSummary {
    /// ECEF [Frame]
    frame: Frame,

    /// One small report per entry
    pub summaries: HashMap<QcSourceDescriptor, QcFileSummary>,
}

impl QcContextSummary {
    /// Initialize a new [QcContextSummary]
    pub fn new(frame: Frame) -> Self {
        Self {
            frame,
            summaries: Default::default(),
        }
    }

    /// Latch [RINEXHeader] in this [QcContextSummary]
    pub fn latch_rinex(&mut self, descriptor: QcSourceDescriptor, data: RINEXHeader) {
        self.summaries.insert(
            descriptor,
            QcFileSummary::RINEX(QcRINEXFileSummary::from_header(&data, self.frame)),
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
