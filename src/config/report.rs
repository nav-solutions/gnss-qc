use serde::{Deserialize, Serialize};

use crate::config::Error;

/// [QcReportType] is used to select the type of report to synthesize.
#[derive(Default, Copy, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QcReportType {
    /// In [Summary] mode, only the summary section
    /// of the report is to be generated. It is the lightest
    /// form we can generate.
    Summary,
    /// In [Full] mode, we generate the [CombinedReport] as well,
    /// which results from the consideration of all input [ProductType]s
    /// at the same time.
    #[default]
    Full,
}

impl std::str::FromStr for QcReportType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "sum" | "summ" | "summary" => Ok(Self::Summary),
            "full" => Ok(Self::Full),
            _ => Err(Error::InvalidReportType),
        }
    }
}

impl std::fmt::Display for QcReportType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Full => f.write_str("Full"),
            Self::Summary => f.write_str("Summary"),
        }
    }
}
