use crate::{
    context::{QcContext, QcDataEntry, QcProductType},
    error::QcError,
    prelude::SP3,
};

use qc_traits::Merge;

use std::path::Path;

use super::QcIndexing;

impl QcContext {
    /// Add this [SP3] into current [QcContext].
    /// NB: we're currenty limited to processing a unique [SP3] publisher (agency).
    /// Future version will allow processing several and offer comparison methods.
    pub fn load_sp3<P: AsRef<Path>>(&mut self, path: P, sp3: SP3) -> Result<(), QcError> {
        let filename = path
            .as_ref()
            .file_stem()
            .ok_or(QcError::FileNameDetermination)?
            .to_string_lossy()
            .to_string();

        // strips possible remaining terminations
        let filename = filename.split('.').collect::<Vec<_>>()[0];

        // data indexing
        let product_type = QcProductType::PreciseOrbit;
        let indexing = QcIndexing::Agency(sp3.header.agency.clone());

        // Add entry
        if let Some(data) = self
            .data
            .iter_mut()
            .filter(|p| p.product_type == product_type && p.indexing == indexing)
            .reduce(|p, _| p)
        {
            let entry = data
                .as_mut_sp3()
                .expect("internal failure: rinex data access");

            entry.merge_mut(&sp3)?;

            debug!("SP3 extension {} - indexed by {}", filename, indexing);
        } else {
            info!("New SP3 {} - indexed by {}", filename, indexing);

            self.data.push(QcDataEntry::new_sp3(filename, sp3));
        }
        Ok(())
    }

    /// Load readable [SP3] file into this [QcContext].
    /// NB: we're currenty limited to processing a unique [SP3] publisher (agency).
    /// So you are expected to load SP3 from the same publisher to obtain correct results.
    /// Future version will allow processing several and offer comparison methods.
    pub fn load_sp3_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), QcError> {
        let sp3 = SP3::from_file(&path)?;
        self.load_sp3(path, sp3)
    }
}
