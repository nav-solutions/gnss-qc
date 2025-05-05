/**
 * All Post Proecessed Navigation support & feature dependent stuff.
 *
 * Authors: Guillaume W. Bres <guillaume.bressaix@gmail.com> et al.
 * (cf. https://github.com/rtk-rs/gnss-qc/graphs/contributors)
 * This framework is shipped under Mozilla Public V2 license.
 *
 * Documentation:
 * - https://github.com/rtk-rs/gnss-qc
 * - https://github.com/rtk-rs/rinex
 * - https://github.com/rtk-rs/sp3
 */
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

use crate::prelude::QcContext;

//pub(crate) mod buffer;
//pub(crate) mod nav_ppp;
// pub(crate) mod time;

// #[cfg(feature = "cggtts")]
// #[cfg_attr(docsrs, doc(cfg(feature = "cggtts")))]
// pub(crate) mod nav_cggtts;

// mod solutions_iter;

//pub use nav_ppp::NavPPPSolver;
//pub use solutions_iter::SolutionsIter;

//use buffer::ephemeris::QcEphemerisData;

// #[cfg(feature = "cggtts")]
// #[cfg_attr(docsrs, doc(cfg(feature = "cggtts")))]
// pub use nav_cggtts::NavCggttsSolver;

// pub use time::NavTimeSolver;

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
    /// ANISE BE440 [MetaFile]
    fn anise_de440s_bsp() -> MetaFile {
        MetaFile {
            crc32: Some(0x7286750a),
            uri: String::from("http://public-data.nyxspace.com/anise/de440s.bsp"),
        }
    }

    /// ANISE PCK11 [MetaFile]
    fn anise_pck11_pca() -> MetaFile {
        MetaFile {
            crc32: Some(0x8213b6e9),
            uri: String::from("http://public-data.nyxspace.com/anise/v0.5/pck11.pca"),
        }
    }

    /// ANISE JPL BPC [MetaFile]
    fn anise_jpl_bpc() -> MetaFile {
        MetaFile {
            crc32: None,
            uri:
                "https://naif.jpl.nasa.gov/pub/naif/generic_kernels/pck/earth_latest_high_prec.bpc"
                    .to_string(),
        }
    }

    /// This [MetaAlmanac] solely relies on the nyx-space servers
    fn default_meta_almanac() -> MetaAlmanac {
        MetaAlmanac {
            files: vec![Self::anise_pck11_pca(), Self::anise_de440s_bsp()],
        }
    }

    /// This [MetaAlmanac] solely relies on the nyx-space servers
    fn high_precision_meta_almanac() -> MetaAlmanac {
        MetaAlmanac {
            files: vec![
                Self::anise_pck11_pca(),
                Self::anise_de440s_bsp(),
                Self::anise_jpl_bpc(),
            ],
        }
    }

    /// Create a new [QcContext] using your own [Almanac] and [Frame] definitions
    /// (obtained externally). NB: [Frame] is supposed to be one of the
    /// Earth Centered Frame as we are supposed to operate on planet Earth.
    /// This is typically used by advanced users targetting high precision naviation.
    pub fn new_alamac_frame(almanac: Almanac, frame: Frame) -> Self {
        Self {
            almanac,
            earth_cef: frame,
            data: Default::default(),
            configuration: Default::default(),
        }
    }

    /// Obtains [Almanac] + ECEF [Frame] definition from ANISE database
    pub(crate) fn default_almanac_frame() -> (Almanac, Frame) {
        let mut meta = Self::default_meta_almanac();

        let almanac = match meta.process(false) {
            Ok(almanac) => almanac,
            Err(e) => {
                error!("anise error: {}", e);
                Almanac::default()
            }
        };

        let frame = almanac
            .frame_from_uid(EARTH_J2000)
            .unwrap_or_else(|e| panic!("anise internal error: {}", e));

        (almanac, frame)
    }

    // // Gather all [QcEphemerisData] available
    // pub fn buffered_ephemeris_data(&self) -> Vec<QcEphemerisData> {
    //     let mut ret = Vec::<QcEphemerisData>::with_capacity(8);

    //     if let Some(brdc) = self.brdc_navigation() {
    //         for (k, v) in brdc.nav_ephemeris_frames_iter() {
    //             if let Some(stored) = QcEphemerisData::from_ephemeris(k.sv, k.epoch, &v) {
    //                 ret.push(stored);
    //             }
    //         }
    //     }

    //     ret
    // }

    /// Update (and possibly upgrade if never used) this [QcContext] for ultra high precision navigation,
    /// using Internet access. The BPC database remains valid for a few weeks. But this should be regularly updated.
    pub fn update_jpl_bpc(&self) -> Result<(), NavigationError> {
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
