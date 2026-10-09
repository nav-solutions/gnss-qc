pub mod an;
pub mod proc;

use thiserror::Error;

/// Errors that may arise during pipeline setting up
#[derive(Debug, Error)]
pub enum QcPipelineSetupError {
    /// Not all operations can be combined and performed in the same run.
    #[error("incompatible operations")]
    IncompatibleOperations,
}
