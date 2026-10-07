#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use hifitime::Epoch;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum QcIndexing {
    /// Indexed by antenna model or alias. Used for signal sources
    Antenna(String),

    /// Indexed by GNSS receivered. Used for signal sources.
    Receiver(String),

    /// Indexed by operator (name or alias), sometimes refered to as
    /// "observer" in RINEX terminology.
    Operator(String),

    /// Indexed by agency name or alias (data publisher). Used for high
    /// precision products.
    Agency(String),

    /// Indexed by custom alias
    Custom(String),

    /// Indexed by file creation date & time.
    /// This only applies to local files.
    FileDateTime(Epoch),
}
