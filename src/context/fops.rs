//! File operations supported by Context, mainly parsing

use crate::prelude::QcContext;

impl QcContext {
    /// Recursively load files one by one from this root [Path].
    /// Specify the max_depth search.
    /// Turn on the logs for file (one by one) log messages.
    /// Returns an [Error] when not a single valid file was picked up,
    /// meaning the context has not been updated.
    /// If only one or a few files are corrupt or invalid, you will need to
    /// browse the log report.
    pub fn load_dir<P: AsRef<Path>>(&mut self, root: P, max_depth: usize) -> Result<(), Error> {
        let mut ret = Err(Error::NonSupportedFormat("no valid file".to_string()));

        let mut walk = WalkDir::new(root).max(max_depth);

        for e in walk {
            match self.load(e) {
                Ok(_) => {
                    ret = Ok(());
                }
                Err(e) => {
                    #[cfg(feature = "logs")]
                    error!(e);
                    ret = Err(Error(e.to_string()));
                }
            }
        }

        ret
    }

    /// Load a single file into this [QcContext].
    /// Use this method to load files one by one.
    /// Otherwise, prefer [QcContext::load_dir].
    /// When loading files one by one, file format must be supported,
    /// otherwise we notify the [QcContext] has not been updated with an [Error].
    /// You might turn on the logs, for parsing information.
    pub fn load<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Error> {
        match self.load_rinex(path) {
            Ok(_) => Ok(()),
            Err(_) => {
                #[cfg(feature = "nav")]
                match self.load_sp3(path) {
                    Ok(_) => Ok(()),
                    Err(_) => Err(Error::NonSupportedFormat),
                }
                #[cfg(not(feature = "nav"))]
                Err(Error::NonSupportedFormat(path.to_string()))
            }
        }
    }


    /// Returns path to File considered as Primary product in this Context.
    /// When a unique file had been loaded, it is obviously considered Primary.
    pub fn primary_path(&self) -> Option<&PathBuf> {
        /*
         * Order is important: determines what format are prioritized
         * in the "primary" determination
         */
        for product in [
            ProductType::Observation,
            // ProductType::DORIS,
            ProductType::BroadcastNavigation,
            ProductType::MeteoObservation,
            // ProductType::IONEX,
            ProductType::ANTEX,
            ProductType::HighPrecisionClock,
            #[cfg(feature = "sp3")]
            ProductType::HighPrecisionOrbit,
        ] {
            if let Some(paths) = self.files(product) {
                /*
                 * Returns Fist file loaded in this category
                 */
                return paths.first();
            }
        }
        None
    }
    
    /// Load [Rinex] file into this [QcContext].
    /// File must be supported and correctly formatted.
    /// Returns an [Error] if this RINEX format is not supported or file is corrupt,
    /// meaning the context has not been updated.
    pub fn load_rinex<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Error> {
        match rinex = Rinex::from_file(&path) {
            Ok(rinex) => {
                trace!("parsed RINEX file \"{}\"", path.to_string());

                match InputType::from(rinex.header.rinex_type) {
                    Ok(input) => {}
                    Err(e) => {
                        error!("RINEX type not supported");
                        Err(Error::NonSupportedFormat)
                    }
                }
            }
            Err(e) => {}
        }

        // extend context blob
        if let Some(paths) = self
            .files
            .iter_mut()
            .filter_map(|(prod, files)| {
                if *prod == prod_type {
                    Some(files)
                } else {
                    None
                }
            })
            .reduce(|k, _| k)
        {
            if let Some(inner) = self.blob.get_mut(&prod_type).and_then(|k| k.as_mut_rinex()) {
                inner.merge_mut(&rinex)?;
                paths.push(path_buf);
            }
        } else {
            self.blob.insert(prod_type, BlobData::RINEX(rinex));
            self.files.insert(prod_type, vec![path_buf]);
        }

        Ok(())
    }
    
    /// Returns reference to files loaded in given category
    pub fn files(&self, product: ProductType) -> Option<&Vec<PathBuf>> {
        self.files
            .iter()
            .filter_map(|(prod_type, paths)| {
                if *prod_type == product {
                    Some(paths)
                } else {
                    None
                }
            })
            .reduce(|k, _| k)
    }

    /// Returns mutable reference to files loaded in given category
    pub fn files_mut(&mut self, product: ProductType) -> Option<&Vec<PathBuf>> {
        self.files
            .iter()
            .filter_map(|(prod_type, paths)| {
                if *prod_type == product {
                    Some(paths)
                } else {
                    None
                }
            })
            .reduce(|k, _| k)
    }

}
