#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::input::cfg::InputConfig;

#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct QcConfig {
    /// Input preferences
    pub input: QcInputConfig,
}
