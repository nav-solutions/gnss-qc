use thiserror::Error;

use log::error;

use anise::{
    almanac::{
        metaload::{MetaAlmanacError, MetaFile},
        planetary::PlanetaryDataError,
    },
    constants::frames::{EARTH_ITRF93, EARTH_J2000},
    errors::AlmanacError,
    prelude::{Almanac, Frame, MetaAlmanac},
};

use crate::{
    navigation::{NavFilter, NavFilterType},
    prelude::{Constellation, QcContext},
};

#[cfg(feature = "navigation")]
use crate::prelude::{Orbit, ReferenceEcefPosition};

#[derive(Debug, Error)]
pub enum NavigationError {
    #[error("almanac error: {0}")]
    Almanac(#[from] AlmanacError),
    #[error("meta error: {0}")]
    MetaAlmanac(#[from] MetaAlmanacError),
    #[error("planetary data error")]
    PlanetaryData(#[from] PlanetaryDataError),
}

impl QcContext {
    /// Returns a possible [ReferenceEcefPosition] if defined in current [QcContext].
    /// NB: this is only picked from a possible [Rinex] Observations, not any
    /// other possible source. If no Observations were loaded, there is no point
    /// asking for this in this current form.
    pub fn reference_rx_position(&self) -> Option<ReferenceEcefPosition> {
        let obs_rinex = self.observation()?;
        let t = obs_rinex.first_epoch()?;
        let rx_orbit = obs_rinex.header.rx_orbit(t, self.earth_cef)?;
        let pos = ReferenceEcefPosition::from_orbit(&rx_orbit);
        Some(pos)
    }

    /// Returns a possible reference position, expressed as [Orbit], if defined in current [QcContext].
    /// NB: this is only picked from a possible [Rinex] Observations, not any
    /// other possible source. If no Observations were loaded, there is no point
    /// asking for this in this current form.
    pub fn reference_rx_orbit(&self) -> Option<Orbit> {
        let obs_rinex = self.observation()?;
        let t = obs_rinex.first_epoch()?;
        obs_rinex.header.rx_orbit(t, self.earth_cef)
    }

    /// Applies complex [NavFilter] to mutable [QcContext].
    pub fn nav_filter_mut(&mut self, filter: &NavFilter) {
        // apply nav conditions
        if let Some(brdc) = self.brdc_navigation_mut() {
            let any_constellation = filter.constellations.is_empty();
            let broad_sbas = filter.constellations.contains(&Constellation::SBAS);

            let brdc_rec = brdc.record.as_mut_nav().unwrap();

            brdc_rec.retain(|k, data| {
                if let Some(eph) = data.as_ephemeris() {
                    match filter.filter {
                        NavFilterType::Healthy => {
                            if k.sv.constellation.is_sbas() && broad_sbas {
                                eph.sv_healthy()
                            } else {
                                if any_constellation {
                                    eph.sv_healthy()
                                } else {
                                    if filter.constellations.contains(&k.sv.constellation) {
                                        eph.sv_healthy()
                                    } else {
                                        true
                                    }
                                }
                            }
                        }
                        NavFilterType::Testing => {
                            if k.sv.constellation.is_sbas() && broad_sbas {
                                eph.sv_in_testing()
                            } else {
                                if any_constellation {
                                    eph.sv_in_testing()
                                } else {
                                    if filter.constellations.contains(&k.sv.constellation) {
                                        eph.sv_in_testing()
                                    } else {
                                        true
                                    }
                                }
                            }
                        }
                        NavFilterType::Unhealthy => {
                            if k.sv.constellation.is_sbas() && broad_sbas {
                                !eph.sv_healthy()
                            } else {
                                if any_constellation {
                                    !eph.sv_healthy()
                                } else {
                                    if filter.constellations.contains(&k.sv.constellation) {
                                        !eph.sv_healthy()
                                    } else {
                                        true
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // preserves other frames
                    true
                }
            });
        }
    }

    /// Upgrade this [QcContext] for ultra high precision navigation.
    pub fn with_jpl_bpc(&self) -> Result<(), NavigationError> {
        let mut s = self.clone();

        let mut meta = Self::high_precision_meta_almanac();
        let almanac = meta.process(true)?;

        s.almanac = almanac;

        let mut meta = Self::default_meta_almanac();
        let almanac = meta.process(true)?;

        let frame = almanac.frame_from_uid(EARTH_ITRF93)?;
        s.earth_cef = frame;

        Ok(())
    }
}
