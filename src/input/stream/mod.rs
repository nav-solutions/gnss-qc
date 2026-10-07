#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

pub mod signal;

use signal::SignalStream;

/// [InputStream] definition, feeding data points to a possible pipeline
pub enum InputStream {
    /// [SignalStream]
    Signal(SignalStream),
}
