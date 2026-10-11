#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::io::input::cfg::QcInputConfig;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct QcConfig {
    /// Input preferences
    pub input: QcInputConfig,
}
