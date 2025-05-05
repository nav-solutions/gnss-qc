mod indexing;
mod product;
mod wrapper;

pub use indexing::QcIndexing;
pub use product::QcProductType;

use wrapper::QcDataWrapper;

use crate::prelude::Rinex;

#[cfg(feature = "sp3")]
use crate::prelude::SP3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QcSourceDescriptor {
    /// Readable filename
    pub filename: String,

    /// Storage indexing as [QcIndexing]
    pub indexing: QcIndexing,

    /// [QcProductType]
    pub product_type: QcProductType,
}

#[derive(Clone)]
pub struct QcDataEntry {
    /// Descriptor
    pub descriptor: QcSourceDescriptor,

    /// Wrapped data as [QcDataWrapper]
    inner: QcDataWrapper,
}

impl QcDataEntry {
    /// Define a new RINEX entry
    /// ## Input
    /// - filename: (readable)
    /// - product_type: [QcProductType]
    /// - indexing: [QcIndexing]
    /// - data: owned [Rinex] data
    pub fn new_rinex(
        filename: &str,
        product_type: QcProductType,
        indexing: QcIndexing,
        data: Rinex,
    ) -> Self {
        Self {
            descriptor: QcSourceDescriptor {
                indexing,
                product_type,
                filename: filename.to_string(),
            },
            inner: QcDataWrapper::RINEX(data),
        }
    }

    /// Returns reference to underlying [Rinex] data
    pub fn as_rinex(&self) -> Option<&Rinex> {
        self.inner.as_rinex()
    }

    /// Returns mutable reference to underlying [Rinex] data
    pub fn as_mut_rinex(&mut self) -> Option<&mut Rinex> {
        self.inner.as_mut_rinex()
    }
}

#[cfg(feature = "sp3")]
#[cfg_attr(docsrs, doc(cfg(feature = "sp3")))]
impl QcDataEntry {
    /// Define a new SP3 entry, always indexed by production agency.
    /// ## Input
    /// - filename: (readable)
    /// - data: owned [SP3] data
    pub fn new_sp3(filename: &str, data: SP3) -> Self {
        let indexing = QcIndexing::Agency(data.header.agency.clone());

        Self {
            descriptor: QcSourceDescriptor {
                indexing,
                filename: filename.to_string(),
                product_type: QcProductType::PreciseOrbit,
            },
            inner: QcDataWrapper::SP3(data),
        }
    }

    /// Returns reference to underlying [SP3] data
    pub fn as_sp3(&self) -> Option<&SP3> {
        self.inner.as_sp3()
    }

    /// Returns mutable reference to underlying [SP3] data
    pub fn as_mut_sp3(&mut self) -> Option<&mut SP3> {
        self.inner.as_mut_sp3()
    }
}
