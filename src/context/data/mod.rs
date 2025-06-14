mod indexing;
mod product;
mod wrapper;
mod match_by;

pub use indexing::QcIndexing;
pub use product::QcProductType;
pub use match_by::QcMatchBy;
pub(crate) use wrapper::QcDataWrapper;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QcSourceDescriptor {
    /// [QcProductType]
    pub product_type: QcProductType,

    /// Storage indexing as [QcIndexing]
    pub indexing: QcIndexing,

    /// Readable filename
    pub filename: String,
}
