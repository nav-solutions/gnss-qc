mod indexing;
mod product;
mod wrapper;

pub use indexing::QcIndexing;
pub use product::QcProductType;
pub(crate) use wrapper::QcDataWrapper;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QcSourceDescriptor {
    /// Readable filename
    pub filename: String,

    /// Storage indexing as [QcIndexing]
    pub indexing: QcIndexing,

    /// [QcProductType]
    pub product_type: QcProductType,
}
