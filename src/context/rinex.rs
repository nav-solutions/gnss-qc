use std::path::Path;

use qc_traits::Merge;

use crate::{
    config::QcPreferedIndexing,
    context::{
        data::{QcData, QcDataWrapper},
        QcDataKey,
    },
    prelude::{QcContext, QcError, QcIndexing, QcProductType, Rinex},
};

use rinex::{
    hardware::{Antenna, Receiver},
    marker::GeodeticMarker,
};

impl QcContext {
    /// Format [Receiver] model
    fn format_gnss_rx(rx: &Receiver) -> String {
        format!("{}-{}", rx.model, rx.sn)
    }

    fn format_geodetic_marker(marker: &GeodeticMarker) -> String {
        if let Some(number) = marker.number() {
            format!("{}-{}", marker.name, number)
        } else {
            format!("{}", marker.name)
        }
    }

    fn format_rx_antenna(antenna: &Antenna) -> String {
        format!("{}-{}", antenna.model, antenna.sn)
    }

    fn auto_rinex_indexing(prod_type: &QcProductType, rinex: &Rinex) -> QcIndexing {
        match prod_type {
            QcProductType::Observation => Self::auto_observation_indexing(&rinex),
            _ => Self::auto_other_rinex_indexing(&rinex),
        }
    }

    fn auto_observation_indexing(rinex: &Rinex) -> QcIndexing {
        if let Some(marker) = &rinex.header.geodetic_marker {
            QcIndexing::GeodeticMarker(Self::format_geodetic_marker(&marker))
        } else if let Some(receiver) = &rinex.header.rcvr {
            QcIndexing::GnssReceiver(Self::format_gnss_rx(&receiver))
        } else if let Some(agency) = &rinex.header.agency {
            QcIndexing::Agency(agency.clone())
        } else if let Some(operator) = &rinex.header.observer {
            QcIndexing::Operator(operator.clone())
        } else if let Some(antenna) = &rinex.header.rcvr_antenna {
            QcIndexing::RxAntenna(Self::format_rx_antenna(&antenna))
        } else {
            QcIndexing::None
        }
    }

    fn auto_other_rinex_indexing(rinex: &Rinex) -> QcIndexing {
        if let Some(agency) = &rinex.header.agency {
            QcIndexing::Agency(agency.clone())
        } else if let Some(operator) = &rinex.header.observer {
            QcIndexing::Operator(operator.clone())
        } else {
            QcIndexing::None
        }
    }

    /// Load a single [Rinex] file into this [QcContext].
    /// File revision must be supported and must be correctly formatted
    /// for this operation to be effective.
    pub fn load_rinex<P: AsRef<Path>>(&mut self, path: P, rinex: Rinex) -> Result<(), QcError> {
        let product_type = QcProductType::from(rinex.header.rinex_type);

        let filename = path
            .as_ref()
            .file_stem()
            .ok_or(QcError::FileNameDetermination)?
            .to_string_lossy()
            .to_string();

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
                    Some(QcIndexing::GnssReceiver(Self::format_gnss_rx(&receiver)))
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
            // manually indexing
            info!("{} manually indexed by {}", filename, indexing);
            indexing
        } else {
            // auto indexing
            let indexing = Self::auto_rinex_indexing(&product_type, &rinex);
            info!("{} auto indexed by {}", filename, indexing);
            indexing
        };

        if let Some(indexed) = self.get_rinex_data_mut(product_type, &indexing) {
            indexed.merge_mut(&rinex)?;
            Ok(())
        } else {
            let key = QcDataKey {
                index: indexing.clone(),
                prod_type: product_type,
            };

            let data = QcData {
                filename: filename,
                inner: QcDataWrapper::RINEX(rinex),
            };

            self.data.insert(key, data);

            Ok(())
        }
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

    /// Returns reference to given RINEX [QcProductType] with desired index
    fn get_rinex_data(&self, prod_type: QcProductType, indexing: &QcIndexing) -> Option<&Rinex> {
        self.rinex_products_iter(prod_type)
            .filter_map(|(index, v)| if index == indexing { Some(v) } else { None })
            .reduce(|k, _| k)
    }

    /// Returns mutable reference to given RINEX [QcProductType] with desired index
    fn get_rinex_data_mut(
        &mut self,
        prod_type: QcProductType,
        indexing: &QcIndexing,
    ) -> Option<&mut Rinex> {
        self.rinex_products_iter_mut(prod_type)
            .filter_map(|(index, v)| if index == indexing { Some(v) } else { None })
            .reduce(|k, _| k)
    }

    /// Returns reference to indexed [QcProductType::Observation] RINEX product (if it exists).
    pub fn get_observation_rinex(&self, indexing: &QcIndexing) -> Option<&Rinex> {
        self.get_rinex_data(QcProductType::Observation, indexing)
    }

    /// Returns mutable reference to indexed [QcProductType::Observation] RINEX product (if it exists).
    pub fn get_observation_rinex_mut(&mut self, indexing: &QcIndexing) -> Option<&mut Rinex> {
        self.get_rinex_data_mut(QcProductType::Observation, indexing)
    }

    /// Returns reference to indexed [QcProductType::MeteoObservation] RINEX product (if it exists).
    pub fn get_meteo_observation_rinex(&self, indexing: &QcIndexing) -> Option<&Rinex> {
        self.get_rinex_data(QcProductType::MeteoObservation, indexing)
    }

    /// Returns mutable reference to indexed [QcProductType::Observation] RINEX product (if it exists).
    pub fn get_meteo_observation_rinex_mut(&mut self, indexing: &QcIndexing) -> Option<&mut Rinex> {
        self.get_rinex_data_mut(QcProductType::MeteoObservation, indexing)
    }

    /// Returns reference to indexed [QcProductType::BroadcastNavigation] RINEX product (if it exists).
    pub fn get_brdc_navigation_rinex(&self, indexing: &QcIndexing) -> Option<&Rinex> {
        self.get_rinex_data(QcProductType::BroadcastNavigation, indexing)
    }

    /// Returns mutable reference to indexed [QcProductType::BroadcastNavigation] RINEX product (if it exists).
    pub fn get_brdc_navigation_rinex_mut(&mut self, indexing: &QcIndexing) -> Option<&mut Rinex> {
        self.get_rinex_data_mut(QcProductType::BroadcastNavigation, indexing)
    }

    /// Returns reference to indexed [QcProductType::PreciseClock] RINEX product (if it exists).
    pub fn get_clock_rinex(&self, indexing: &QcIndexing) -> Option<&Rinex> {
        self.get_rinex_data(QcProductType::PreciseClock, indexing)
    }

    /// Returns mutable reference to indexed [QcProductType::PreciseClock] RINEX product (if it exists).
    pub fn get_clock_rinex_mut(&mut self, indexing: &QcIndexing) -> Option<&mut Rinex> {
        self.get_rinex_data_mut(QcProductType::PreciseClock, indexing)
    }
}
