use std::path::Path;

use crate::{error::QcError, prelude::QcContext, context::QcMatchBy};

impl QcContext {
    /// [QcContext::synthesize] generates output product
    /// from the current [QcContext]. This is typically
    /// used after preprocessing to generate new data.
    ///
    /// ## Input
    /// - directory: [Path] prefix
    /// - match_by: Optional [QcMatchBy] to restrict data to synthesize
    ///
    /// Example (1): synthesize all possible products after preprocessing
    /// ```
    /// ```
    ///
    /// Example (2): synthesize RINEX products only, from a PPP compliant context
    /// ```
    /// ```
    ///
    /// Example (3): synthesize products from one particular receiver only,
    /// from RTK network:
    /// ```
    /// ```
    pub fn synthesize<P: AsRef<Path>>(
        &self,
        directory: P,
        match_by: Option<QcMatchBy>,
    ) -> Result<(), QcError> {
        for (desc, data) in self.data.iter() {
            let mut matched = match_by.is_none();

            if let Some(match_by) = match_by {
                // matched |= match_by.match(desc);
            }

            if matched {}
        }

        Ok(())
    }

    /// Applies [Self::synthesize] to all/any product types and files contained in
    /// current [QcContext].
    ///
    /// ## Input
    /// - directory: [Path] prefix
    ///
    /// Example:
    /// ```
    /// ```
    ///
    /// Refer to [Self::synthesize] for more information
    pub fn synthesize_all<P: AsRef<Path>>(&self, directory: P) -> Result<(), QcError> {
        self.synthesize(directory, None)
    }
}
