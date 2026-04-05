GNSS Quality Control (Qc)
=========================

[![Rust](https://github.com/nav-solutions/gnss-qc/actions/workflows/rust.yml/badge.svg)](https://github.com/nav-solutions/gnss-qc/actions/workflows/rust.yml)
[![Rust](https://github.com/nav-solutions/gnss-qc/actions/workflows/daily.yml/badge.svg)](https://github.com/nav-solutions/gnss-qc/actions/workflows/daily.yml)
[![crates.io](https://docs.rs/gnss-qc/badge.svg)](https://docs.rs/gnss-qc/)
[![crates.io](https://img.shields.io/crates/d/gnss-qc.svg)](https://crates.io/crates/gnss-qc)

[![MRSV](https://img.shields.io/badge/MSRV-1.82.0-orange?style=for-the-badge)](https://github.com/rust-lang/rust/releases/tag/1.82.0)
[![License](https://img.shields.io/badge/license-MPL_2.0-orange?style=for-the-badge&logo=mozilla)](https://github.com/nav-solutions/qc-traits/blob/main/LICENSE)

The GNSS Quality Control (QC) is a core library to facilitate GNSS and geodesic post-processing
algotithms, such as precise navigation, usually working with text files line RINEX as an input.

The library offers a 'Context' object, capable to contain most common user input and provides
convenient iteration methods. The context is formed by grabing data points from all supported input files:

- RINEX (navigation, orbits, observations, meteo sensors).
This format is provided by my [RINEX parser](https://github.com/nav-solutions/rinex).

- SP3 (precise orbits)

Not only that, but it relies heavly on great external ecosystems:

- ANISE: allows us to attach Ephemeris and Planteray states, including
projections and rotations, mostly used in navigation processes
- HIFITIME: for timescale and epochs definition
- GEO: for projection and basic calculations

Other interesting features:

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
