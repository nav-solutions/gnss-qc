GNSS Quality Control
====================

[![Rust](https://github.com/rtk-rs/gnss-qc/actions/workflows/rust.yml/badge.svg)](https://github.com/rtk-rs/gnss-qc/actions/workflows/rust.yml)
[![Rust](https://github.com/rtk-rs/gnss-qc/actions/workflows/daily.yml/badge.svg)](https://github.com/rtk-rs/gnss-qc/actions/workflows/daily.yml)
[![crates.io](https://docs.rs/gnss-qc/badge.svg)](https://docs.rs/gnss-qc/)
[![crates.io](https://img.shields.io/crates/d/gnss-qc.svg)](https://crates.io/crates/gnss-qc)

[![MRSV](https://img.shields.io/badge/MSRV-1.81.0-orange?style=for-the-badge)](https://github.com/rust-lang/rust/releases/tag/1.81.0)
[![License](https://img.shields.io/badge/license-MPL_2.0-orange?style=for-the-badge&logo=mozilla)](https://github.com/rtk-rs/qc-traits/blob/main/LICENSE)

The GNSS Quality Control (QC) library is an advanced library that proposes
from basic up to advanced GNSS and Geodesy processing features. Since the spectrum
of GNSS applications is broad, so is `gnss_qc`. Amongst several, you may use this library for the
following purpose:

* Process RINEX files
* Process SP3 files (on `sp3` lib feature)
* Consider what RINEX + SP3 combination has to offer
* Obtain Radio based orbital projections (on `navigation` lib feature)
* Obtain SP3 orbital projection (on `navigation` lib feature)
* Perform SP3 versus radio based residual analysis
* Consider precise Clock products
* Preprocess all supported products, in particular
  * data filtering
  * decimation (down sampling)
  * focus on data of interest
  * transpose to different TimeScale
* Resolve precise P.V.T (Position, Velocity, Time) solutions
by deploying the `NavPPP` solver (on `navigation` lib feature)
* Resolve precise CGGTTS solutions
by deploying a `NavCggtts` solver (on `navigation` lib feature)

It is made possible by the complex combination of several frameworks and libraries.
It is important to understand this library's features & options.

Licensing
=========

This library is part of the [RTK-rs framework](https://github.com/rtk-rs) which
is delivered under the [Mozilla V2 Public](https://www.mozilla.org/en-US/MPL/2.0) license.

## Core level

The fundammental blocks that we rely upon, at all times

- [Hifitime by Nyx-Space](https://github.com/nyx-space/hifitime) 
that provides Epoch and TimeScale definitions
- [GNSS by RTK-rs](https://github.com/rtk-rs/qc-traits) that provides
Constellation and SV definitions
- [Qc Traits by RTK-rs](https://github.com/rtk-rs/qc-traits) that provides 
shared behavior by all GNSS libraries
- [The RINEX library by RTK-rs](https://github.com/rtk-rs/rinex) because we consider
the RINEX files as the most fundamental. It is currently not possible to build
this library without RINEX support (say: SP3 only application). But that could easily be changed.

## Basic and default features

- `flate2` is activated by default, and allows Gzip compressed files to be naturally supported.
- `sp3` is activated by default, because we consider people interested in GNSS post processing
are interested in high precision at all times. This is easily changed by de-activating this crate feature.

## Navigation feature

`navigation` is the most advanced feature. It allows post processed navigation and is the heaviest option.
This option relies on 

- [ANISE by Nyx-Space](https://github.com/nyx-space/anise) 
- [GNSS-RTK by RTK-rs](https://github.com/rtk-rs/gnss-rtk)

If you are only interested in file processing and management, you should not activate Post Processed navigation support.

This will unlock very high level yet very advanced `NavPvtSolver` to resolve and gather PVT solutions from
any provided Context!

```rust
use gnss_qc::prelude::{
    QcContext, 
    NavPreset, 
    NavMethod, 
    NavUserProfile,
    NavSolutionsIter,
};

let mut ctx = QcContext::new();

// Load some data (static geodetic marker).
// We will survey this position. 
// If a position was not described, this process would be the actual process of obtaining this marker.
ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
    .unwrap();

// Data for that day
ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
    .unwrap();
 
// Select a general preset, that mostly defines the selected navigation method.
let preset = NavPreset::static_preset(NavMethod::CPP);

// Deploy the solver.
let mut ppp = ctx.nav_ppp_solver(preset)
    .expect("This context is navigation compatible!");

// In static applications like this example,
// only evolution of the measurement systems may apply here.
// In this basic demo, we consider it remained the same for that entire session.
let user_profile = NavUserProfile::default();

// PVT solutions are collected in chronological order, by iterating the solver.
while let Some(output) = ppp.next(user_profile) {
    match output {
        Ok(pvt) => {
            // Resolved a solution
        },
        Err(e) => {
            // Something went wront internally.
            // This is mostoften neglicted, especially when you are
            // confident about your presets.
            // You can also study the error to make some decisions.
        },
    }
}
```

## Deploying without navigation support

Without navigation support, this library will allow GNSS context creation and basic processing.
You will not access the most advanced solvers.

## Embedded ephemeris data

Use the `embed_ephem` library feature to download the basic ephemeris data at build time.
Otherwise, if you intend post-processed navigation, your first deployment (ever) will require
acess to the Internet. This is only needed as long as the cached files are preserved.

High Precision Navigation
=========================

The solutions you may resolve only depends on the input products. You control the deployment setups at all times. From the previous example, we simply stack an SP3 file, declare it as prefered source, and now resolve PPP solutions:

```rust
use gnss_qc::prelude::QcContext;

let mut ctx = QcContext::new();

// Load some data (static geodetic marker).
// We will survey this position. 
// If a position was not described, this process would be the actual process of obtaining this marker.
ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
    .unwrap();

// Data for that day
ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
    .unwrap();

// Precise laboratory data for that day
ctx.load_gzip_sp3_file("data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz")
    .unwrap();

// Deploy the solver.
let mut nav_ppp = ctx.nav_pvt_solver()
    .expect("This context is navigation compatible!");
```

CGGTTS solutions
================

Activate `cggtts` option to unlock the `nav_cggtts_ppp_solver` method, associated solver and structures.  
This one operates very similarly as `NavPvtSolver` and is dedicated to CGGTTS solutions solving.

This option relies on [CGGTTS by RTK-rs](https://github.com/rtk-rs/cggtts):

```rust
use gnss_qc::prelude::QcContext;

let mut ctx = QcContext::new();

// Load some data
ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
    .unwrap();

ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
    .unwrap();

// Deploy the solver, using PPP navigation technique.
let mut nav_cggtts = ctx.nav_cggtts_ppp_solver()
    .expect("This context is navigation compatible!");
```
