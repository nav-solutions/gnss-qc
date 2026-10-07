use crate::input::types::InputProductType;
use thiserror::Error;

use std::path::Path;

#[derive(Debug, Error)]
pub enum IndexingError {

}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct InputProductKey {
    /// Name for this input product. For static text files
    /// like RINEX files, this is simply the file name.
    pub name: String,
    
    /// Alias used in complex indexing method.
    pub alias: String,

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
        let fname = path.file_name()?;
        
        Ok(Self {
            name: fname.to_string(),
            alias: fname.to_string(),
            product_type: InputProductType::RINEX,
        })
    }
    
}
