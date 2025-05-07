#![doc(html_logo_url = "https://raw.githubusercontent.com/rtk-rs/.github/master/logos/logo2.jpg")]
#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

/*
 * GNSS-Qc is part of the rtk-rs framework.
 * Authors: Guillaume W. Bres <guillaume.bressaix@gmail.com> et al.
 * (cf. https://github.com/rtk-rs/gnss-qc/graphs/contributors)
 * This framework is shipped under Mozilla Public V2 license.
 *
 * Documentation:
 * - https://github.com/rtk-rs/gnss-qc
 * - https://github.com/rtk-rs/rinex
 * - https://github.com/rtk-rs/sp3
 */

#[cfg(feature = "navigation")]
#[macro_use]
extern crate log;

extern crate gnss_qc_traits as qc_traits;
extern crate gnss_rs as gnss;

pub(crate) mod analysis;
mod config;
mod context;
// mod pipeline;
// mod plot;
// mod html;
mod report;
mod serializer;

#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
mod navigation;

pub mod error;

#[cfg(test)]
mod tests;

pub mod prelude {
    pub use crate::{
        config::{QcConfig, QcPreferedIndexing},
        context::{QcContext, QcIndexing, QcProductType},
        error::QcError,
    };

    // GNSS re-export
    pub use gnss::prelude::{Constellation, COSPAR, SV};

    pub use qc_traits::{
        Filter, FilterItem, GnssAbsoluteTime, MaskOperand, Preprocessing, Repair, RepairTrait,
        TimePolynomial, Timeshift,
    };

    // Hifitime re-export
    pub use hifitime::prelude::{Duration, Epoch, TimeScale};

    // RINEX re-export
    pub use rinex::prelude::{Error as RinexError, Rinex};

    pub use std::path::Path;

    #[cfg(feature = "html")]
    pub use maud::Markup;

    #[cfg(feature = "sp3")]
    pub use sp3::prelude::{Error as SP3Error, SP3};

    #[cfg(feature = "navigation")]
    pub use crate::{
        config::QcOrbitPreference,
        navigation::{QcNavFilter, QcNavFilterType, ReferenceEcefPosition},
    };

    #[cfg(feature = "navigation")]
    pub use anise::{
        constants::frames::{EARTH_ITRF93, EARTH_J2000, IAU_EARTH_FRAME, SUN_J2000},
        prelude::{Almanac, Frame, Orbit},
    };

    // #[cfg(feature = "navigation")]
    // pub use gnss_rtk::prelude::{Config as NavPreset, Method as NavMethod, User as NavUserProfile};

    // #[cfg(feature = "cggtts")]
    // pub use crate::context::navigation::NavCggttsSolver;
}
