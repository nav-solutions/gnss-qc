GNSS Quality Control
====================

[![Rust](https://github.com/rtk-rs/gnss-qc/actions/workflows/rust.yml/badge.svg)](https://github.com/rtk-rs/gnss-qc/actions/workflows/rust.yml)
[![Rust](https://github.com/rtk-rs/gnss-qc/actions/workflows/daily.yml/badge.svg)](https://github.com/rtk-rs/gnss-qc/actions/workflows/daily.yml)
[![crates.io](https://docs.rs/gnss-qc/badge.svg)](https://docs.rs/gnss-qc/)
[![crates.io](https://img.shields.io/crates/d/gnss-qc.svg)](https://crates.io/crates/gnss-qc)

[![MRSV](https://img.shields.io/badge/MSRV-1.81.0-orange?style=for-the-badge)](https://github.com/rust-lang/rust/releases/tag/1.81.0)
[![License](https://img.shields.io/badge/license-MPL_2.0-orange?style=for-the-badge&logo=mozilla)](https://github.com/rtk-rs/qc-traits/blob/main/LICENSE)

The GNSS Quality Control (QC) library is an advanced library that proposes
from basic to advanced GNSS and Geodesy processing pipelines.

It is made possible by the complex combination of several frameworks and libraries.
It is important to understand this library's features & options.

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
use gnss_qc::prelude::QcContext;

let mut ctx = QcContext::new();

// Load some data
ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
    .unwrap();

ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
    .unwrap();

let mut nav_pvt = ctx.nav_pvt_solver()
    .expect("This context is navigation compatible!");
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

// Load some data
ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
    .unwrap();

ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
    .unwrap();

// Context is now PPP compatible
ctx.load_gzip_sp3_file("data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz")
    .unwrap();

let mut nav_ppp = ctx.nav_pvt_solver()
    .expect("This context is navigation compatible!");
```

CGGTTS solutions
================

Activate `cggtts` option to lock the `nav_cggtts_solver` method, associated solver and structures.  
This one operates very similarly as `NavPvtSolver` and is dedicated to CGGTTS solutions solving.

This option relies on [CGGTTS by RTK-rs](https://github.com/rtk-rs/cggtts):

```rust
use gnss_qc::prelude::QcContext;

// Load some data
ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
    .unwrap();

ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
    .unwrap();

let mut nav_cggtts = ctx.nav_cggtts_solver()
    .expect("This context is navigation compatible!");
```
