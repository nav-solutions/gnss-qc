//! Qc Analysis module

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// [QcAtmosphericAnalysis] regroups all feabible atmospheric analysis.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum QcAtmosphericAnalysis {
    /// Serialize and stream out all
    /// meteo sensor observations from all available sensors
    Observations,
}

/// [QcSignalAnalysis] regroups all feabible signal analysis.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum QcSignalAnalysis {
    /// Serialize and stream out all
    /// signal observations from all available sources
    Observations,
}

#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum QcAnalysis {
    /// [QcSignalAnalysis]
    Signal(QcSignalAnalysis),

    /// [QcAtmosphericAnalysis]
    Atm(QcAtmosphericAnalysis),

    /// Serialize, resolve and stream out all satellite coordinates
    /// from input streams.
    #[cfg(feature = "nav")]
    Kepler,
}

pub struct QcAnalyzer {
    analysis: Vec<QcAnalysis>,
}

impl QcAnalyzer {
    pub fn new(analysis: Vec<QcAnalysis>) {
        Self { analysis }
    }
}
