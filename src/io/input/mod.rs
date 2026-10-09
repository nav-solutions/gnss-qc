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
    /// When arriving from unknown file format, the library will try
    /// every single supported (and _compiled_) format. We wind up here
    /// when every single format attempt has failed which either means:
    /// - the core has not been compiled for this format
    /// - one internal core dependency is unable to parse this input correctly
    /// - the input is not correctly formatted and is invalid according
    /// to the standards
    #[error("file format not supported")]
    FormatNotSupported,

    /// When arriving from a local file, we may use the file name for
    /// internal determination, depending on input product
    #[error("failed to determine file name (stem)")]
    FileStemIssue,

    /// When loading static files into the product table (prior processing),
    /// input file formats must be supported. In case any parser fails, we will wind up here.
    #[error("file type not supported {0}")]
    FileNotSupported(String),

    /// When a classification method has been selected (infaillible for each supported format),
    /// the new entry must be merged into the existing table, extending the table with new data
    /// points. This error rises if the extension step fails for some reason.
    #[error("rinex file stacking issue: {0}")]
    RinexMerge(MergeError),
}
