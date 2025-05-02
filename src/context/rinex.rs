use std::{
    collections::hash_map::{Iter as HashMapIter, IterMut as HashMapIterMut},
    path::Path,
};

use qc_traits::Merge;

use crate::{
    config::QcPreferedIndexing,
    prelude::{QcContext, QcError, QcIndexing, QcProductType, Rinex},
};

use rinex::{
    hardware::{Antenna, Receiver},
    marker::GeodeticMarker,
};

use log::{debug, info};

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

    fn auto_indexing(rinex: &Rinex) -> QcIndexing {
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

        // Observation RINEX are uniquely indexed.
        // Any other types are still mixed up.
        // Which facilitates the post-processing.
        match product_type {
            QcProductType::Observation => {
                self.load_observation_rinex(filename, rinex)?;
            }
            QcProductType::MeteoObservation => {
                if let Some(meteo) = &mut self.meteo_observations {
                    meteo.merge_mut(&rinex)?;
                } else {
                    self.meteo_observations = Some(rinex);
                }
            }
            QcProductType::BroadcastNavigation => {
                if let Some(brdc) = &mut self.brdc_navigation {
                    brdc.merge_mut(&rinex)?;
                } else {
                    self.brdc_navigation = Some(rinex);
                }
            }
            QcProductType::PreciseClock => {
                if let Some(clock) = &mut self.precise_clocks {
                    clock.merge_mut(&rinex)?;
                } else {
                    self.precise_clocks = Some(rinex);
                }
            }
            QcProductType::IONEX => {
                if let Some(ionex) = &mut self.ionex {
                    ionex.merge_mut(&rinex)?;
                } else {
                    self.ionex = Some(rinex);
                }
            }
            QcProductType::DORIS => panic!("DORIS observations not supported yet"),
            QcProductType::ANTEX => panic!("ANTEX records not supported yet"),
            _ => unreachable!("other types!"),
        }

        Ok(())
    }

    fn load_observation_rinex(&mut self, filename: String, rinex: Rinex) -> Result<(), QcError> {
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
            let indexing = Self::auto_indexing(&&rinex);
            info!("{} auto indexed by {}", filename, indexing);
            indexing
        };

        if let Some(indexed) = self
            .observations
            .iter_mut()
            .filter_map(
                |(index, data)| {
                    if *index == indexing {
                        Some(data)
                    } else {
                        None
                    }
                },
            )
            .reduce(|k, _| k)
        {
            debug!("{} - extension", indexing);
            indexed.merge_mut(&rinex)?;
            Ok(())
        } else {
            debug!("{} - new entry", indexing);
            self.observations.insert(indexing, rinex);
            Ok(())
        }
    }

    /// Load a readable [Rinex] file into this [QcContext].
    pub fn load_rinex_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), QcError> {
        let rinex = Rinex::from_file(&path)?;
        self.load_rinex(path, rinex)
    }

    /// Returns reference to all [QcProductType::Observation] RINEX products,
    /// accross all data sources.
    pub fn observation_sources_iter(&self) -> HashMapIter<'_, QcIndexing, Rinex> {
        self.observations.iter()
    }

    /// Returns reference to all [QcProductType::Observation] RINEX products,
    /// accross all data sources.
    pub fn observation_sources_iter_mut(&mut self) -> HashMapIterMut<'_, QcIndexing, Rinex> {
        self.observations.iter_mut()
    }

    /// Returns reference to [QcProductType::Observation] for this data source
    pub fn observations_data(&self, data_source: &QcIndexing) -> Option<&Rinex> {
        self.observation_sources_iter()
            .filter_map(|(source, data)| {
                if source == data_source {
                    Some(data)
                } else {
                    None
                }
            })
            .reduce(|k, _| k)
    }

    /// Returns mutable reference to [QcProductType::Observation] for this data source
    pub fn observations_data_mut(&mut self, data_source: &QcIndexing) -> Option<&mut Rinex> {
        self.observation_sources_iter_mut()
            .filter_map(|(source, data)| {
                if source == data_source {
                    Some(data)
                } else {
                    None
                }
            })
            .reduce(|k, _| k)
    }

    /// Returns true if [QcProductType::Observation] is present in current [QcContext],
    /// whatever the data source.
    pub fn has_observations(&self) -> bool {
        for (_, _) in self.observation_sources_iter() {
            return true;
        }
        false
    }

    /// Returns true if [QcProductType::BroadcastNavigation] is present in the current [QcContext].
    pub fn has_brdc_navigation(&self) -> bool {
        self.brdc_navigation.is_some()
    }

    /// Returns reference to [QcProductType::BroadcastNavigation] data, if preset in the current [QcContext].
    pub fn brdc_navigation_data(&self) -> Option<&Rinex> {
        self.brdc_navigation.as_ref()
    }

    /// Returns mutable reference to [QcProductType::BroadcastNavigation] data, if preset in the current [QcContext].
    pub fn brdc_navigation_data_mut(&mut self) -> Option<&mut Rinex> {
        self.brdc_navigation.as_mut()
    }

    /// Returns true if [QcProductType::MeteoObservation] is present in the current [QcContext].
    pub fn has_meteo_observations(&self) -> bool {
        self.meteo_observations.is_some()
    }

    /// Returns reference to [QcProductType::MeteoObservation] data, if preset in the current [QcContext].
    pub fn meteo_observations_data(&self) -> Option<&Rinex> {
        self.meteo_observations.as_ref()
    }

    /// Returns mutable reference to [QcProductType::MeteoObservation] data, if preset in the current [QcContext].
    pub fn meteo_observations_data_mut(&mut self) -> Option<&mut Rinex> {
        self.meteo_observations.as_mut()
    }

    /// Returns reference to [QcProductType::PreciseClock] data, if preset in the current [QcContext].
    pub fn precise_clock_data(&self) -> Option<&Rinex> {
        self.precise_clocks.as_ref()
    }

    /// Returns mutable reference to [QcProductType::MeteoObservation] data, if preset in the current [QcContext].
    pub fn precise_clock_data_mut(&mut self) -> Option<&mut Rinex> {
        self.meteo_observations.as_mut()
    }

    /// Returns reference to [QcProductType::IONEX] data, if preset in the current [QcContext].
    pub fn ionex_data(&self) -> Option<&Rinex> {
        self.ionex.as_ref()
    }

    /// Returns reference to [QcProductType::IONEX] data, if preset in the current [QcContext].
    pub fn ionex_data_mut(&mut self) -> Option<&mut Rinex> {
        self.ionex.as_mut()
    }
}
