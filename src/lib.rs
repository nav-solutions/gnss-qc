#![doc(
    html_logo_url = "https://raw.githubusercontent.com/nav-solutions/.github/master/logos/logo2.jpg"
)]
#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

/*
 * GNSS-Qc is part of the NAV-Solutions framework.
 * Authors: Guillaume W. Bres <guillaume.bressaix@gmail.com> et al.
 * (cf. https://github.com/nav-solutions/gnss-qc/graphs/contributors)
 * This framework is shipped under Mozilla Public V2 license.
 *
 * Documentation:
 * - https://github.com/nav-solutions/gnss-qc
 * - https://github.com/nav-solutions/rinex
 * - https://github.com/nav-solutions/sp3
 * - https://github.com/nav-solutions/cggtts
 */

#[cfg(feature = "navigation")]
#[macro_use]
extern crate log;

extern crate gnss_qc_traits as qc_traits;
extern crate gnss_rs as gnss;

pub mod io;

#[cfg(test)]
mod tests;

pub mod prelude {}
