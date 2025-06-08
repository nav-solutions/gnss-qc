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

#[cfg(feature = "sp3")]
mod sp3_data;

#[cfg(feature = "sp3")]
pub use sp3_data::QcPreciseState;

#[derive(Debug)]
pub struct QcSerializedData<'a, T> {
    /// origin filename
    pub filename: &'a str,

    /// Product type source
    pub product_type: QcProductType,

    /// Method used in indexing
    pub indexing: &'a QcIndexing,

    /// Type dependent data
    pub data: T,
}

pub type QcSerializedEphemeris<'a> = QcSerializedData<'a, QcEphemerisData>;

pub type QcSerializedSignal<'a> = QcSerializedData<'a, QcSignalData>;

pub type QcSerializedRINEXHeader<'a> = QcSerializedData<'a, &'a RINEXHeader>;

#[cfg(feature = "sp3")]
pub type QcSerializedSP3Header<'a> = QcSerializedData<'a, &'a SP3Header>;

#[cfg(feature = "sp3")]
pub type QcSerializedPreciseState<'a> = QcSerializedData<'a, QcPreciseState>;

pub enum QcSerializedItem<'a> {
    /// [QcSerializedRINEXHeader]
    RINEXHeader(QcSerializedRINEXHeader<'a>),

    #[cfg(feature = "sp3")]
    /// [QcSerializedSP3Header]
    SP3Header(QcSerializedSP3Header<'a>),

    /// [QcSerializedSignal]
    Signal(QcSerializedSignal<'a>),

    /// [QcEphemerisData]
    Ephemeris(QcSerializedEphemeris<'a>),

    #[cfg(feature = "sp3")]
    /// [QcSerializedPreciseState]
    PreciseState(QcSerializedPreciseState<'a>),
}
