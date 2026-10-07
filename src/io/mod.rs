mod input;
mod output;

use input::QcInputError;

/// I/O errors, whether they are file I/O or not is case dependent.
#[derive(Debug, Error)]
pub enum QcIoErrors {
    Input(QcInputErrors),
}
