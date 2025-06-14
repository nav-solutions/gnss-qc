use crate::{
    context::{QcContext, QcDataWrapper, QcProductType, QcSourceDescriptor},
    error::QcError,
    prelude::SP3,
};

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
        info!("New SP3 \"{}\" - indexed by {}", filename, indexing);

        let descriptor = QcSourceDescriptor {
            filename: filename.to_string(),
            indexing,
            product_type,
        };

        self.data.insert(descriptor, QcDataWrapper::SP3(sp3));
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

    /// Obtain an [Iterator] over all SP3 products that were loaded
    pub fn sp3_filenames_iter(&self) -> Box<dyn Iterator<Item = String> + '_> {
        Box::new(self.data.iter().filter_map(|(k, _)| {
            if k.product_type == QcProductType::PreciseOrbit {
                Some(k.filename.clone())
            } else {
                None
            }
        }))
    }

    /// Obtain an [Iterator] over all SP3 products from this agency
    pub fn sp3_agency_filenames_iter(&self, agency: &str) -> Box<dyn Iterator<Item = String> + '_> {
        let filter = QcIndexing::from_agency(agency);

        Box::new(self.data.iter().filter_map(|(k, _)| {
            if k.product_type == QcProductType::PreciseOrbit && k.indexing == filter {
                Some(k.filename.clone())
            } else {
                None
            }
        }))
    }

    /// Returns total number of SP3 [QcProductType]s that were loaded
    pub fn total_sp3_files(&self) -> usize {
        self.sp3_filenames_iter().count()
    }

    /// Within all SP3 products, delete this source file.
    /// Does not affect other types of products.
    /// NB: the file termination is not preserved by this library.
    /// To be sure, use the available context and file iterators.
    pub fn sp3_delete_filename(&mut self, sp3_filename: &str) {
        self.data.retain(|desc, _| {
            match desc.product_type {
                QcProductType::PreciseOrbit => desc.filename != sp3_filename,
                _ => true,
            }
        });
    }
    
    /// Within all SP3 products, delete those published by this agency.
    pub fn sp3_delete_agency(&mut self, agency: &str) {
        self.data.retain(|desc, _| {
            match desc.product_type {
                QcProductType::PreciseOrbit => {
                    match desc.indexing {
                        QcIndexing::Agency(ag) => ag != agency,
                        _ => true,
                    }
                },
                _ => true,
            }
        });
    }

    /// From all SP3 products, retain only this source file.
    /// Does not affect other types of products.
    /// NB: the file termination is not preserved by this library.
    /// To be sure, use the available context and file iterators.
    pub fn sp3_retain_filename(&mut self, sp3_filename: &str) {
        self.data.retain(|desc, _| {
            match desc.product_type {
                QcProductType::PreciseOrbit => desc.filename == sp3_filename,
                _ => true,
            }
        });
    }

    /// Within all SP3 products, retain only those published by this agency.
    pub fn sp3_retain_agency(&mut self, agency: &str) {
        self.data.retain(|desc, _| {
            match desc.product_type {
                QcProductType::PreciseOrbit => {
                    match desc.indexing {
                        QcIndexing::Agency(ag) => ag == agency,
                        _ => false,
                    }
                },
                _ => true,
            }
        });
    }
}
