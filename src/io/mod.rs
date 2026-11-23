pub mod input;
pub mod output;
pub mod source;

use input::QcInputError;

use thiserror::Error;

/// I/O errors, whether they are file I/O or not is case dependent.
#[derive(Debug, Error)]
pub enum QcIoErrors {
    #[error("input error: {0}")]
    Input(QcInputError),
}
