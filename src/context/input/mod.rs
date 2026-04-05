use thiserror::Error;

pub mod types;

#[derive(Debug, Error)]
pub enum Error {
    /// Non supported format (with filename)
    NonSupportedFormat,
}

pub struct InputProduct {
    /// Supported [ProductType]
    pub product_type: types::ProductType,

    /// Readable file directory
    pub parent_dir: String,

    /// Readable File name
    pub file_name: String,

    /// Pointer for browsing
    pointer: InputPointer,
}

impl InputProduct {
    /// "Reset" this InputProduct.
    /// Most products are indexed in chronological order, this 
    /// means reseting to first epoch.
    pub fn reset_mut(&mut self) {
        self.pointer.reset_mut();
    }
}

/// Input products provided by User
pub struct InputProducts {
    /// Input products provided by User
    inner: Vec<InputProduct>,     
}

impl InputProducts {
    pub fn load(&mut self, product_type: types::ProductType, parent: String, name: String) {
        self.inner.push(
            InputProduct {

            });
    }

}
