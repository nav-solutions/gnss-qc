use crate::input::types::InputProductType;
use thiserror::Error;

use std::path::Path;

use hifitime::prelude::Epoch;

#[derive(Debug, Error)]
pub enum IndexingError {
    /// InputFileName could not be determined (OS issue)
    #[error("could not determine input file name")]
    InputFileName,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct InputProductKey {
    /// Name for this input product. For static text files
    /// like RINEX files, this is simply the file name.
    pub name: String,

    /// Alias used in complex indexing method.
    pub alias: String,

    /// File creation date & time
    pub datetime: Epoch,
    
    /// Type of attached product
    pub product_type: InputProductType,
}

impl InputProductKey {
    /// Guess [InputProductKey] indexing method from a local file [Path].
    /// This is not the most efficient classification technique because it will
    /// attempt each supported data format (one by one) and select the one that passes internally,
    /// and we deduce the product type from there.
    pub fn guess_from_path<P: AsRef<Path>>(path: P) -> Result<Self, IndexingError> {
        let path = path.as_ref();

        let name = path.file_name()
            .ok_or(IndexingError::InputFileName)?
            .to_str()
            .ok_or(IndexingError::InputFileName)?;

        let meta = path.file_meta()
            .ok_or(IndexingError::InputFileMeta)?;

        let datetime = meta.created()
            .map_err(|_| IndexingError::InputFileMeta)?;

        let name_str = name.to_string();
        
        Ok(Self {
            name: name_str,
            alias: name_str.clone(),
            datetime: {
                 
            },
            product_type: InputProductType::RINEX,
        })
    }
    
}
