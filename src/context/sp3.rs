use crate::{
    context::{
        data::{QcData, QcDataWrapper},
        QcDataKey, QcIndexing,
    },
    error::QcError,
    prelude::{QcContext, QcProductType, SP3},
};

use qc_traits::Merge;

use std::path::Path;

impl QcContext {
    /// Add this [SP3] into current [QcContext].
    /// File revision must be supported and must be correctly formatted
    /// for this operation to be effective.
    pub fn load_sp3<P: AsRef<Path>>(&mut self, path: P, sp3: SP3) -> Result<(), QcError> {
        let filename = path
            .as_ref()
            .file_stem()
            .ok_or(QcError::FileNameDetermination)?
            .to_string_lossy()
            .to_string();

        // SP3 is always indexed by publisher name, which always exists in correctly formed SP3.
        let agency = &sp3.header.agency;

        if let Some(indexed) = self.get_sp3_by_agency_mut(&agency) {
            indexed.merge_mut(&sp3)?;
            Ok(())
        } else {
            let key = QcDataKey {
                index: QcIndexing::Agency(agency.to_string()),
                prod_type: QcProductType::PreciseOrbit,
            };

            let data = QcData {
                filename,
                inner: QcDataWrapper::SP3(sp3),
            };

            self.data.insert(key, data);

            Ok(())
        }
    }

    /// Load readable [SP3] file into this [QcContext].
    pub fn load_sp3_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), QcError> {
        let sp3 = SP3::from_file(&path)?;
        self.load_sp3(path, sp3)
    }

    /// Returns reference to all inner SP3 products
    /// ```
    /// use gnss_qc::prelude::{QcContext, QcIndexing, QcProductType};
    ///
    /// // create a new (empty) context with default setup
    /// let mut context = QcContext::new();
    ///
    /// // [QcPreferedIndexing] method does not apply to SP3
    /// context.configuration.indexing = QcPreferedIndexing::GnssReceiver;
    ///
    /// // load some data
    /// context.load_sp3_file("data/SP3/C/co108870.sp3")
    ///     .unwrap();
    ///
    /// // this becomes true
    /// assert!(context.has_sp3_data());
    ///
    /// // this file reports onboard clock data as well
    /// assert!(context.has_sp3_clock_data());
    ///
    /// // SP3 data is always correctly indexed, even by default.
    /// for (publisher, sp3) in context.sp3_agencies_product_iter() {
    ///     assert_eq!(publisher, "AIUB");   
    /// }
    ///
    /// // No data published by IGS was loaded here
    /// ```
    pub fn sp3_agencies_iter(&self) -> Box<dyn Iterator<Item = (String, &SP3)> + '_> {
        Box::new(
            self.products_iter(QcProductType::PreciseOrbit)
                .filter_map(|(index, v)| {
                    if let Some(sp3) = v.inner.as_sp3() {
                        let agency = index.as_agency().unwrap_or_else(|| {
                            panic!("internal error: should not happen with valid SP3 data")
                        });

                        Some((agency, sp3))
                    } else {
                        None
                    }
                }),
        )
    }

    /// [QcContext::sp3_agencies_iter] mutable implementation
    pub fn sp3_agencies_iter_mut(&mut self) -> Box<dyn Iterator<Item = (String, &mut SP3)> + '_> {
        Box::new(
            self.products_iter_mut(QcProductType::PreciseOrbit)
                .filter_map(|(index, v)| {
                    if let Some(sp3) = v.inner.as_mut_sp3() {
                        let agency = index.as_agency().unwrap_or_else(|| {
                            panic!("internal error: should not happen with valid SP3 data")
                        });

                        Some((agency, sp3))
                    } else {
                        None
                    }
                }),
        )
    }

    /// Returns true if at least one [QcProductType::PreciseOrbit] is present in current [QcContext].
    pub fn has_sp3_data(&self) -> bool {
        for (_, _) in self.sp3_agencies_iter() {
            return true;
        }
        false
    }

    /// Returns true if at least one [QcProductType::PreciseOrbit] is present in current [QcContext]
    /// and reports onboard clock data.
    pub fn has_sp3_clock_data(&self) -> bool {
        for (_, sp3) in self.sp3_agencies_iter() {
            if sp3.has_satellite_clock_offset() {
                return true;
            }
        }

        false
    }

    /// Returns true if [QcProductType::PreciseOrbit] published by given agency,
    /// does report onboard clock data.
    pub fn has_agency_sp3_clock_data(&self, agency: &str) -> bool {
        for (publisher, sp3) in self.sp3_agencies_iter() {
            if publisher == agency {
                if sp3.has_satellite_clock_offset() {
                    return true;
                }
            }
        }

        false
    }

    /// Returns reference to inner [SP3] data published by this agency.
    /// ```
    /// use gnss_qc::prelude::{QcContext, QcIndexing, QcProductType};
    ///
    /// // create a new (empty) context with default setup
    /// let mut context = QcContext::new();
    ///
    /// // [QcPreferedIndexing] method does not apply to SP3
    /// context.configuration.indexing = QcPreferedIndexing::GnssReceiver;
    ///
    /// // load some data
    /// context.load_sp3_file("data/SP3/C/co108870.sp3")
    ///     .unwrap();
    ///
    /// assert!(context.get_sp3_by_agency("AIUB").is_some());
    /// assert!(context.get_sp3_by_agency("IGS").is_none());
    /// ```
    pub fn get_sp3_by_agency(&self, agency: &str) -> Option<&SP3> {
        self.sp3_agencies_iter()
            .filter_map(
                |(publisher, sp3)| {
                    if publisher == agency {
                        Some(sp3)
                    } else {
                        None
                    }
                },
            )
            .reduce(|k, _| k)
    }

    /// [QcContext::get_sp3_by_agency] mutable implementation.
    pub fn get_sp3_by_agency_mut(&mut self, agency: &str) -> Option<&mut SP3> {
        self.sp3_agencies_iter_mut()
            .filter_map(
                |(publisher, sp3)| {
                    if publisher == agency {
                        Some(sp3)
                    } else {
                        None
                    }
                },
            )
            .reduce(|k, _| k)
    }
}
