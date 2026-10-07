//! Qc pipeline input
pub mod cfg;
pub mod indexing;
pub mod key;
pub mod product;
pub mod stream;
pub mod types;

use qc_traits::MergeError;
use thiserror::Error;

use indexing::QcIndexingError;

/// Input I/O error, whether they are file I/O or not is case dependent.
#[derive(Debug, Error)]
pub enum QcInputError {
    /// Error during (static/local) input product indexing.
    /// Local files are indexed to be classified in the table.
    /// A classification method must be selected for each entry.
    /// For most files, we can select

    /// When a classification method has been selected (infaillible for each supported format),
    /// the new entry must be merged into the existing table, extending the table with new data
    /// points. This error rises if the extension step fails for some reason.
    #[error("rinex file stacking issue: {0}")]
    RinexMerge(MergeError),

    /// When loading static files into the product table (prior processing),
    /// input file formats must be supported. In case any parser fails, we will wind up here.
    #[error("file type not supported {0}")]
    FileNotSupported(String),
}
