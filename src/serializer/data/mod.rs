// Serialized data
use crate::context::{QcIndexing, QcProductType};

mod ephemeris;
mod header;
mod signal;

pub use ephemeris::QcEphemerisData;
pub use signal::{QcSignalData, QcSignalObservation};

use rinex::prelude::Header as RINEXHeader;

#[cfg(feature = "sp3")]
use sp3::prelude::Header as SP3Header;

#[derive(Clone)]
pub struct QcSerializedData<T: Clone> {
    /// origin filename
    pub filename: String,

    /// Product type source
    pub product_type: QcProductType,

    /// Method used in indexing
    pub indexing: QcIndexing,

    /// Type dependent data
    pub data: T,
}

pub type QcSerializedEphemeris = QcSerializedData<QcEphemerisData>;

pub type QcSerializedSignal = QcSerializedData<QcSignalData>;

pub type QcSerializedRINEXHeader = QcSerializedData<RINEXHeader>;

#[cfg(feature = "sp3")]
pub type QcSerializedSP3Header = QcSerializedData<SP3Header>;

#[derive(Clone)]
pub enum QcSerializedItem {
    /// [QcSerializedRINEXHeader]
    RINEXHeader(QcSerializedRINEXHeader),

    #[cfg(feature = "sp3")]
    /// [QcSerializedSP3Header]
    SP3Header(QcSerializedSP3Header),

    /// [QcSerializedSignalData]
    Signal(QcSerializedSignal),

    /// [QcEphemerisData]
    Ephemeris(QcSerializedEphemeris),
}
