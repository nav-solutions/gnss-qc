GNSS Quality Control (Qc)
=========================

[![Rust](https://github.com/nav-solutions/gnss-qc/actions/workflows/rust.yml/badge.svg)](https://github.com/nav-solutions/gnss-qc/actions/workflows/rust.yml)
[![Rust](https://github.com/nav-solutions/gnss-qc/actions/workflows/daily.yml/badge.svg)](https://github.com/nav-solutions/gnss-qc/actions/workflows/daily.yml)

[![crates.io](https://img.shields.io/crates/d/gnss-qc.svg)](https://crates.io/crates/gnss-qc)
[![crates.io](https://docs.rs/gnss-qc/badge.svg)](https://docs.rs/gnss-qc/)
[![discord server](https://img.shields.io/discord/1342922474110586910?logo=discord)](https://discord.gg/EqhEBXBmJh)

[![MRSV](https://img.shields.io/badge/MSRV-1.89.0-orange?style=for-the-badge)](https://github.com/rust-lang/rust/releases/tag/1.89.0)
[![License](https://img.shields.io/badge/license-MPL_2.0-orange?style=for-the-badge&logo=mozilla)](https://github.com/nav-solutions/qc-traits/blob/main/LICENSE)

The GNSS Quality Control (QC) libary is a core library to faciliate GNSS and geodesic post-processing.   
It tries to wrap in a single library, the possibility to combine and mix several input, usually
text files like RINEX or SP3, and deploy advanced algorithms from there. The ultimage objective
being precise navigation. 

It also tries to fulfill what the historical `teqc` program achieves.

This library is released under [AGPLv3](https://www.gnu.org/licenses/agpl-3.0.en.html).

We propose one implementation of this framework:

- [RINEX-Cli (command line)](https://github.com/nav-solutions/rinex-cli)
which exposes most of the options offered by this framework. It supports the same
compilation options and is automatically released for Linux and Windows.

Library features
================

Support of the RINEX format (readable or compressed) is native, because we consider this format to be
the backbone and most common format used in post-processing.
It relies on our advanced [RINEX parser](https://github.com/nav-solutions/rinex), which
supports file compresion, basic analysis, several SERDES operations and integrates a modern rewrite
of the CRINEX implementation.

Support for all other formats are feature dependent and use a compilation flag closely related or directly
derived from that format. Activating specific format has advanced effects, possibly subtle to new comers.
For example, precise navigation implicitely means the SP3 products should be loaded so that feature is expected.
The library strives to expose everything that is physically possible based on the user context and compilation options.

- `flate2`: is activated by default and allows parsing Gzip compressed files natively.
We consider people use Gzip compressed files 99% of the time, which is most convenient when exchanging data recordings.
This applies to RINEX, SP3 data files.

- `sp3`: activate support for precise orbit products (IGS): precise navigation will not really be feasible
without this feature. At best it will take a lot longer to reach the same solution quality, or it will simply not be feasible.

- `navigation`: post-processed navigation is not feasible without this feature. It is the most complex
feature we propose. This application enables features dedicated to post-processed navigation.
It will automatically integrate our [GNSS-RTK Solver](https://github.com/nav-solutions/gnss-rtk) which

QcContext
=========

Amongst all objects contained in this library, the QcContext is the most fundamental.

External libraries
==================

External libraries that are vital to this project:

- ANISE: allows us to attach Ephemeris and Planteray states, including
projections and rotations, mostly used in navigation processes
- HIFITIME: for timescale and epochs definition
- GEO: for projection and basic calculations
- Nalgebra

- `flate2`, activated by default, and allows to parse Gzip compressed files directly

This library is part of the [NAV-Solutions framework](https://github.com/nav-solutions) 
and is licensed under the [Mozilla V2 Public](https://www.mozilla.org/en-US/MPL/2.0) license.

## Terminology

The GNSS context is a binary blob that one can serialize, deserialize, store and retrieve from backup conveniently.
This is possible thanks to the great serdes capabilities offered by Rust.

We refer to "User Input" as the input products one can provide to form such a context.
Once the context is formed, one can iterate (browse) and post process the dataset by using
one of the methodes provided by this library: this is referred to as post-processing.

This library is post-processing oriented and does not match the requirements of true real-time processing.
One of the main reason is that we only work with input files and we cannot connect to a receiver.
If you are interested in real-time navigation, we have a demo application that can tie to a UBlox receiver,
it is called [rt-navi](https://github.com/nav-sls/rt-navi).

We refer to "Output products" as the solutions the algorithms will output. Most solutions
are once again binary blobs, for which we provide a few format: HTML mostly, but we hope to provide
PDF support as well.

## Precise navigation

The `nav` (navigation) feature is the most advanced feature.
`nav` is the most advanced feature. It allows post processed navigation and is the heaviest option.
This option relies on [ANISE by Nyx-Space](https://github.com/nyx-space/anise).

If you are only interested in file processing and management, you should not activate Post Processed navigation support.

## Deploying without navigation support

Without navigation support, this library will allow GNSS context creation and basic processing.
You will not access the most advanced solvers.
