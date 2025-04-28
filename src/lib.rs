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

mod config;
mod context;
mod product;
mod report;

#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
mod navigation;

pub mod error;
pub mod plot;

#[cfg(test)]
mod tests;

pub mod prelude {
    pub use crate::{
        config::{orbit::QcOrbitPreference, report::QcReportType, QcConfig},
        context::QcContext,
        error::Error,
        product::ProductType,
        report::{QcExtraPage, QcReport},
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

    pub use crate::plot::{Marker, MarkerSymbol, Mode, Plot};
    pub use maud::{html, Markup, Render};

    #[cfg(feature = "sp3")]
    pub use sp3::prelude::{Error as SP3Error, SP3};

    #[cfg(feature = "navigation")]
    pub use crate::{
        context::navigation::NavPPPSolver,
        navigation::{NavFilter, NavFilterType, ReferenceEcefPosition},
    };

    #[cfg(feature = "navigation")]
    pub use anise::prelude::{Almanac, Frame, Orbit};

    #[cfg(feature = "cggtts")]
    pub use crate::context::navigation::NavCggttsSolver;
}
