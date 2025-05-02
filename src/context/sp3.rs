use crate::{
    error::QcError,
    prelude::{QcContext, SP3},
};

use qc_traits::Merge;

use std::path::Path;

impl QcContext {
    /// Add this [SP3] into current [QcContext].
    /// NB: we're currenty limited to processing a unique [SP3] publisher (agency).
    /// Future version will allow processing several and offer comparison methods.
    pub fn load_sp3(&mut self, sp3: SP3) -> Result<(), QcError> {
        // SP3 are not indexed by publisher as of today.
        if let Some(inner) = &mut self.sp3 {
            inner.merge_mut(&sp3)?;
        } else {
            self.sp3 = Some(sp3);
        }

        Ok(())
    }

    /// Load readable [SP3] file into this [QcContext].
    /// NB: we're currenty limited to processing a unique [SP3] publisher (agency).
    /// So you are expected to load SP3 from the same publisher to obtain correct results.
    /// Future version will allow processing several and offer comparison methods.
    pub fn load_sp3_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), QcError> {
        let sp3 = SP3::from_file(&path)?;
        self.load_sp3(sp3)
    }

    /// Returns true if at least one [QcProductType::PreciseOrbit] is present in current [QcContext].
    pub fn has_sp3_data(&self) -> bool {
        self.sp3.is_some()
    }

    /// Returns true if at least one [QcProductType::PreciseOrbit] is present in current [QcContext]
    /// and reports onboard clock data.
    pub fn has_sp3_clock_data(&self) -> bool {
        if let Some(sp3) = &self.sp3 {
            sp3.has_satellite_clock_offset()
        } else {
            false
        }
    }
}
