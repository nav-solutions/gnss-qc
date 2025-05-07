use std::path::Path;

use qc_traits::Merge;

use crate::{
    config::QcPreferedIndexing,
    context::{QcContext, QcDataEntry, QcIndexing, QcProductType},
    prelude::{QcError, Rinex},
};

use log::info;

impl QcContext {
    /// Load a single [Rinex] file into this [QcContext].
    /// File revision must be supported and must be correctly formatted
    /// for this operation to be effective.
    pub fn load_rinex<P: AsRef<Path>>(&mut self, path: P, rinex: Rinex) -> Result<(), QcError> {
        let filename = path
            .as_ref()
            .file_stem()
            .ok_or(QcError::FileNameDetermination)?
            .to_string_lossy()
            .to_string();

        // strips possible remaining terminations
        let filename = filename.split('.').collect::<Vec<_>>()[0];

        // data indexing
        let product_type = QcProductType::from(rinex.header.rinex_type);

        let indexing = match self.configuration.indexing {
            QcPreferedIndexing::Agency => {
                if let Some(agency) = &rinex.header.agency {
                    Some(QcIndexing::Agency(agency.clone()))
                } else {
                    warn!("No agency found for \"{}\"", filename);
                    None
                }
            }
            QcPreferedIndexing::GnssReceiver => {
                if let Some(receiver) = &rinex.header.rcvr {
                    Some(QcIndexing::from_receiver(receiver))
                } else {
                    warn!("No receiver model defined in \"{}\"", filename);
                    None
                }
            }
            QcPreferedIndexing::Operator => {
                if let Some(operator) = &rinex.header.observer {
                    Some(QcIndexing::Operator(operator.clone()))
                } else {
                    warn!("No operator name defined in \"{}\"", filename);
                    None
                }
            }
            QcPreferedIndexing::Auto => None,
        };

        let indexing = if let Some(indexing) = indexing {
            // manual preference
            indexing
        } else {
            // automated
            let indexing = QcIndexing::rinex_indexing(&rinex);

            // little more smartness when it comes to Broadcast Navigation
            // Although it is not really needed, it will give us the ability
            // to differentiate potential initial "sources" (sampler) of the navigation messages.
            // But this requires the filename to follow standard naming conventions.
            if indexing == QcIndexing::None && product_type == QcProductType::BroadcastNavigation {
                let production = &rinex.production;

                // Will be marked as agency, but we could use "Custom" as well here
                let indexing = QcIndexing::Agency(production.name.clone());
                indexing
            } else {
                indexing
            }
        };

        // Add entry
        if let Some(data) = self
            .data
            .iter_mut()
            .filter(|p| {
                p.descriptor.product_type == product_type && p.descriptor.indexing == indexing
            })
            .reduce(|p, _| p)
        {
            let entry = data
                .as_mut_rinex()
                .expect("internal failure: rinex data access");

            entry.merge_mut(&rinex)?;

            debug!(
                "{} RINEX extension \"{}\" - indexed by {}",
                product_type, filename, indexing
            );
        } else {
            info!(
                "New {} RINEX \"{}\" - indexed by {}",
                product_type, filename, indexing
            );

            self.data.push(QcDataEntry::new_rinex(
                filename,
                product_type,
                indexing,
                rinex,
            ));
        }

        Ok(())
    }

    /// Load a readable [Rinex] file into this [QcContext].
    pub fn load_rinex_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), QcError> {
        let rinex = Rinex::from_file(&path)?;
        self.load_rinex(path, rinex)
    }

    /// Obtain an [Iterator] over all RINEX [QcProductType]s present in current [QcContext].
    pub fn rinex_products_iter(&self) -> Box<dyn Iterator<Item = QcProductType> + '_> {
        Box::new(self.product_types_iter().filter(|p| p.is_rinex_product()))
    }

    /// Returns total number of RINEX [QcProductType]s that were loaded
    pub fn total_rinex_products(&self) -> usize {
        self.rinex_products_iter().count()
    }

    /// Obtain an [Iterator] over all RINEX files that were loaded
    pub fn rinex_filenames_iter(&self) -> Box<dyn Iterator<Item = String> + '_> {
        Box::new(self.data.iter().filter_map(|p| {
            if p.descriptor.product_type != QcProductType::PreciseOrbit {
                Some(p.descriptor.filename.clone())
            } else {
                None
            }
        }))
    }

    /// Returns total number of SP3 [QcProductType]s that were loaded
    pub fn total_rinex_files(&self) -> usize {
        self.rinex_filenames_iter().count()
    }

    /// Returns reference to internal Observation RINEX data, if present in current [QcContext]
    /// for matching data source indexing.
    pub fn rinex_observation_data(&self, indexing: &QcIndexing) -> Option<&Rinex> {
        self.data
            .iter()
            .filter_map(|p| {
                if p.descriptor.product_type == QcProductType::Observation
                    && p.descriptor.indexing == *indexing
                {
                    p.as_rinex()
                } else {
                    None
                }
            })
            .reduce(|k, _| k)
    }

    /// Returns reference to internal Navigation RINEX data, if present in current [QcContext]
    /// for matching data source indexing.
    pub fn rinex_navigation_data(&self, indexing: &QcIndexing) -> Option<&Rinex> {
        self.data
            .iter()
            .filter_map(|p| {
                if p.descriptor.product_type == QcProductType::BroadcastNavigation
                    && p.descriptor.indexing == *indexing
                {
                    p.as_rinex()
                } else {
                    None
                }
            })
            .reduce(|k, _| k)
    }

    /// Returns reference to internal Meteo RINEX data, if present in current [QcContext]
    /// for matching data source indexing.
    pub fn rinex_meteo_data(&self, indexing: &QcIndexing) -> Option<&Rinex> {
        self.data
            .iter()
            .filter_map(|p| {
                if p.descriptor.product_type == QcProductType::MeteoObservation
                    && p.descriptor.indexing == *indexing
                {
                    p.as_rinex()
                } else {
                    None
                }
            })
            .reduce(|k, _| k)
    }
}
