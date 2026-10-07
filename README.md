Geodetic processing pipeline & Quality Control
==============================================

[![Rust](https://github.com/nav-solutions/gnss-qc/actions/workflows/rust.yml/badge.svg)](https://github.com/nav-solutions/gnss-qc/actions/workflows/rust.yml)
[![Rust](https://github.com/nav-solutions/gnss-qc/actions/workflows/daily.yml/badge.svg)](https://github.com/nav-solutions/gnss-qc/actions/workflows/daily.yml)
[![crates.io](https://docs.rs/gnss-qc/badge.svg)](https://docs.rs/gnss-qc/)
[![crates.io](https://img.shields.io/crates/d/gnss-qc.svg)](https://crates.io/crates/gnss-qc)

[![MRSV](https://img.shields.io/badge/MSRV-1.82.0-orange?style=for-the-badge)](https://github.com/rust-lang/rust/releases/tag/1.82.0)
[![License](https://img.shields.io/badge/license-MPL_2.0-orange?style=for-the-badge&logo=mozilla)](https://github.com/nav-solutions/qc-traits/blob/main/LICENSE)

The GNSS Quality Control (`gnss-qc`) is an advanced (pure Rust) library that 
to answer the complex requirements of geodesy processing pipelines.

This library is designed to answer the requirements for complex geodetic processing pipelines,
that demand dealing with several data sources, of different format, sometimes statics or not,
and deploy algorithms like post-processed navigations to obtain solutions.

`gnss-qc` is not designed for post-processed applications specifically, and is not limited to
static input sources. In particular, it takes advantage of the great Tokio framework
to propose abstract and real time slots, which will apply to users that want to perform real time processing.

`gnss-qc` is also flexible and exposes a few sets of Traits, which leaves the possibility for the users
to provide their custom data sources. It does not have to be integrated to this library to actually be usable
as a valid data source.

`gnss-qc` is not limited to input text files
- RTCM binary streams will be supported (on RTCM crate feature)
- BINEX streams will be supported (on BINEX crate feature)
- UBX streams will be supported (on UBX crate feature)
- Raw GNSS binary streams will be supported (on `protos` crate feature)

Also, `gnss-qc` only streams the solutions of the preselected algorithm, it is up to the user to display
and continue their processing. The idea is to propose an efficient, compelling and easy to use library
that does the complex stuff for you. Also, data viewing and projection is once again very dependent on the use case.
People interested in large datasets will need to process many more results and will have to make design choices.
While users only interested in basic quality control don't have such requirements. `gnss-qc` "deals" with that
by leaving the solution up to the final user.

## Framework

`gnss-qc` achieve this complex task by taking advantage of several key elements and frameworks

- `gnss-qc` is fully part of the [NAV-solutions framework]
- `gnss-qc` is capable of deploying our [GNSS-RTK solver](https://github.com/nav-solutions/gnss-rtk) which
covers the need to process P.V.T solutions
- The library relies on the [ANISE](https://github.com/nyx-space/anise) core for solar system astrodynamics and projections
- Our framework relis on [Hifitime](https://github.com/nyx-space/hifitime) for time scale definitions and processing

## Licensing

This library is part of the [NAV-Solutions framework](https://github.com/nav-solutions) 
and is licensed under the [Mozilla V2 Public](https://www.mozilla.org/en-US/MPL/2.0) license.

## Crate features

`gnss-qc` uses crate features extensively, to adapt to the user requirements. The most complex the user
requirements, the heaviest the library. The most advanced features being the combination of the `nav` and `cggtts` features.

When compiled without any options, `gnss-qc` 

Note that the CRINEX format is supported natively and is not tied to a specific feature.

## Applications

This very framework proposes two implemnetations of the `gnss-qc` core:
- `rinex-cli` which is dedicated to static RINEX files: merging, patching and fixing.
- `sp3-cli` which is dedicated to SP3 files: merging..
