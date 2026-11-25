#[cfg(feature = "ionex")]
pub mod ionex;

pub mod input;
pub mod output;
pub mod rinex;

#[cfg(feature = "rtcm")]
#[cfg_attr(docsrs, doc(cfg(feature = "rtcm")))]
pub mod rtcm;

#[cfg(feature = "ntrip")]
#[cfg_attr(docsrs, doc(cfg(feature = "ntrip")))]
pub mod ntrip;

pub mod source;
#[cfg(feature = "sp3")]
pub mod sp3;

use input::QcInputError;

use thiserror::Error;

/// I/O errors, whether they are file I/O or not is case dependent.
#[derive(Debug, Error)]
pub enum QcIoErrors {
    #[error("input error: {0}")]
    Input(QcInputError),
}
