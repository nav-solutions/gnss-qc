use crate::prelude::{QcContext, QcError, QcIndexing, QcProductType, Rinex};
use std::path::Path;

#[cfg(doc)]
use crate::prelude::QcPreferedIndexing;

impl QcContext {
    /// Load a single [Rinex] file into this [QcContext].
    /// File revision must be supported and must be correctly formatted
    /// for this operation to be effective.
    pub fn load_rinex<P: AsRef<Path>>(&mut self, path: P, rinex: Rinex) -> Result<(), QcError> {
        let prod_type = QcProductType::from(rinex.header.rinex_type);

        let path_buf = path.as_ref().to_path_buf();

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

    /// Returns reference to all inner RINEX products matching desired [QcProductType]
    /// ```
    /// use gnss_qc::prelude::{QcContext, QcIndexing, QcProductType};
    ///
    /// // create a new (empty) context with default setup
    /// let mut context = QcContext::new();
    ///
    /// // load some data
    /// context.load_rinex_file("data/OBS/V2/AJAC3550.21O")
    ///     .unwrap();
    ///
    /// // by default, this framework uses a smart indexing method.
    ///
    /// // broad RINEX product search
    /// for (indexing, rinex_product) in context.rinex_products_iter() {
    ///     assert_eq!(indexing, QcIndexing::GnssReceiver);   
    /// }
    /// ```
    ///
    /// When a prefered indexing method is selected, we will try to use it.
    /// In this example (OBS/V2/AJAC35550), the following would apply correct:
    /// - [QcPreferedIndexing::GnssReceiver] "LEICA GR50 2090088" as already demonstrated and used by default
    /// - [QcPreferedIndexing::Agency] "IGN"
    /// - [QcPreferedIndexing::Operator] "Automatic"
    /// - [QcPreferedIndexing::GeodeticMarker] "AJAC-10077M005"
    pub fn rinex_products_iter(
        &self,
        product: QcProductType,
    ) -> Box<dyn Iterator<Item = (&QcIndexing, &Rinex)> + '_> {
        Box::new(self.products_iter(product).filter_map(|(k, v)| {
            if let Some(rinex) = v.inner.as_rinex() {
                Some((k, rinex))
            } else {
                None
            }
        }))
    }

    /// Returns mutable reference to all inner RINEX products matching desired [QcProductType]
    /// ```
    /// use gnss_qc::prelude::{QcContext, QcIndexing, QcProductType};
    ///
    /// // create a new (empty) context with default setup
    /// let mut context = QcContext::new();
    ///
    /// // load some data
    /// context.load_rinex_file("data/OBS/V2/AJAC3550.21O")
    ///     .unwrap();
    ///
    /// // by default, this framework uses a smart indexing method.
    ///
    /// // broad RINEX product search
    /// for (indexing, rinex_product) in context.rinex_products_iter_mut() {
    ///     assert_eq!(indexing, QcIndexing::GnssReceiver);   
    /// }
    /// ```
    ///
    /// When a prefered indexing method is selected, we will try to use it.
    /// In this example (OBS/V2/AJAC35550), the following would apply correct:
    /// - [QcPreferedIndexing::GnssReceiver] "LEICA GR50 2090088" as already demonstrated and used by default
    /// - [QcPreferedIndexing::Agency] "IGN"
    /// - [QcPreferedIndexing::Operator] "Automatic"
    /// - [QcPreferedIndexing::GeodeticMarker] "AJAC-10077M005"
    pub fn rinex_products_iter_mut(
        &mut self,
        product: QcProductType,
    ) -> Box<dyn Iterator<Item = (&QcIndexing, &mut Rinex)> + '_> {
        Box::new(self.products_iter_mut(product).filter_map(|(k, v)| {
            if let Some(rinex) = v.inner.as_mut_rinex() {
                Some((k, rinex))
            } else {
                None
            }
        }))
    }

    /// Returns reference to all [QcProductType::Observation] RINEX products.
    pub fn observations_rinex_iter(&self) -> Box<dyn Iterator<Item = (&QcIndexing, &Rinex)> + '_> {
        self.rinex_products_iter(QcProductType::Observation)
    }

    /// Returns mutable reference to all [QcProductType::Observation] RINEX products.
    pub fn observations_rinex_iter_mut(
        &mut self,
    ) -> Box<dyn Iterator<Item = (&QcIndexing, &mut Rinex)> + '_> {
        self.rinex_products_iter_mut(QcProductType::Observation)
    }

    /// Returns reference to all [QcProductType::BroadcastNavigation] RINEX products.
    pub fn brdc_navigations_rinex_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (&QcIndexing, &Rinex)> + '_> {
        self.rinex_products_iter(QcProductType::BroadcastNavigation)
    }

    /// Returns mutable reference to all [QcProductType::BroadcastNavigation] RINEX products.
    pub fn brdc_navigations_rinex_iter_mut(
        &mut self,
    ) -> Box<dyn Iterator<Item = (&QcIndexing, &mut Rinex)> + '_> {
        self.rinex_products_iter_mut(QcProductType::BroadcastNavigation)
    }

    /// Returns reference to all [QcProductType::PreciseClock] RINEX products.
    pub fn clocks_rinex_iter(&self) -> Box<dyn Iterator<Item = (&QcIndexing, &Rinex)> + '_> {
        self.rinex_products_iter(QcProductType::PreciseClock)
    }

    /// Returns mutable reference to all [QcProductType::PreciseClock] RINEX products.
    pub fn clocks_rinex_iter_mut(
        &mut self,
    ) -> Box<dyn Iterator<Item = (&QcIndexing, &mut Rinex)> + '_> {
        self.rinex_products_iter_mut(QcProductType::PreciseClock)
    }

    /// Returns reference to indexed [QcProductType::Observation] RINEX product (if it exists).
    pub fn get_observation_rinex(&self, indexing: QcIndexing) -> Option<&Rinex> {
        self.rinex_products_iter(QcProductType::Observation)
            .filter_map(|(index, v)| if *index == indexing { Some(v) } else { None })
            .reduce(|k, _| k)
    }

    /// Returns mutable reference to indexed [QcProductType::Observation] RINEX product (if it exists).
    pub fn get_observation_rinex_mut(&mut self, indexing: QcIndexing) -> Option<&mut Rinex> {
        self.rinex_products_iter_mut(QcProductType::Observation)
            .filter_map(|(index, v)| if *index == indexing { Some(v) } else { None })
            .reduce(|k, _| k)
    }

    /// Returns reference to indexed [QcProductType::BroadcastNavigation] RINEX product (if it exists).
    pub fn get_brdc_navigation_rinex(&self, indexing: QcIndexing) -> Option<&Rinex> {
        self.rinex_products_iter(QcProductType::BroadcastNavigation)
            .filter_map(|(index, v)| if *index == indexing { Some(v) } else { None })
            .reduce(|k, _| k)
    }

    /// Returns mutable reference to indexed [QcProductType::BroadcastNavigation] RINEX product (if it exists).
    pub fn get_brdc_navigation_rinex_mut(&mut self, indexing: QcIndexing) -> Option<&mut Rinex> {
        self.rinex_products_iter_mut(QcProductType::BroadcastNavigation)
            .filter_map(|(index, v)| if *index == indexing { Some(v) } else { None })
            .reduce(|k, _| k)
    }

    /// Returns reference to indexed [QcProductType::PreciseClock] RINEX product (if it exists).
    pub fn get_clock_rinex(&self, indexing: QcIndexing) -> Option<&Rinex> {
        self.rinex_products_iter(QcProductType::PreciseClock)
            .filter_map(|(index, v)| if *index == indexing { Some(v) } else { None })
            .reduce(|k, _| k)
    }

    /// Returns mutable reference to indexed [QcProductType::PreciseClock] RINEX product (if it exists).
    pub fn get_clock_rinex_mut(&mut self, indexing: QcIndexing) -> Option<&mut Rinex> {
        self.rinex_products_iter_mut(QcProductType::PreciseClock)
            .filter_map(|(index, v)| if *index == indexing { Some(v) } else { None })
            .reduce(|k, _| k)
    }
}
