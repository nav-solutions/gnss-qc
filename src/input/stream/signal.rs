#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};



/// [InputStream] definition, feeding data points to a possible pipeline
pub enum InputStream {
    /// [SignalStream]
    Signal(SignalStream),

}
