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

pub mod error;

pub(crate) mod serializer;

mod config;
mod context;
mod processing;
mod report;

#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
mod navigation;

#[cfg(test)]
mod tests;

pub mod prelude {
    pub use crate::{
        config::{QcConfig, QcPreferedIndexing},
        context::{QcContext, QcIndexing, QcProductType},
        error::QcError,
        // processing::analysis::QcAnalysisBuilder,
    };

    // GNSS re-export
    pub use gnss::prelude::{Constellation, COSPAR, SV};

    pub use qc_traits::{
        Decimate, DecimationFilter, Filter, FilterItem, MaskFilter, MaskOperand,
        Masking, Preprocessing, Repair, RepairTrait, Split, TimeCorrectionsDB, TimeCorrection, TimeCorrectionError, Timeshift,
    };

    // Hifitime re-export
    pub use hifitime::prelude::{Duration, Epoch, TimeScale};

    // RINEX re-export
    pub use rinex::prelude::{Error as RinexError, Rinex};

    pub use std::path::Path;

    #[cfg(feature = "sp3")]
    pub use sp3::prelude::{Error as SP3Error, SP3};

    #[cfg(feature = "navigation")]
    pub use crate::{
        config::QcOrbitPreference,
        navigation::{QcNavFilter, QcNavFilterType, QcReferencePosition},
    };

    #[cfg(feature = "navigation")]
    pub use anise::{
        constants::frames::{EARTH_ITRF93, EARTH_J2000, IAU_EARTH_FRAME, SUN_J2000},
        prelude::{Almanac, Frame, Orbit},
    };
}
