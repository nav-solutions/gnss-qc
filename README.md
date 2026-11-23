GNSS Quality Control
====================

[![Rust](https://github.com/nav-solutions/gnss-qc/actions/workflows/rust.yml/badge.svg)](https://github.com/nav-solutions/gnss-qc/actions/workflows/rust.yml)
[![Rust](https://github.com/nav-solutions/gnss-qc/actions/workflows/daily.yml/badge.svg)](https://github.com/nav-solutions/gnss-qc/actions/workflows/daily.yml)
[![crates.io](https://docs.rs/gnss-qc/badge.svg)](https://docs.rs/gnss-qc/)
[![crates.io](https://img.shields.io/crates/d/gnss-qc.svg)](https://crates.io/crates/gnss-qc)

[![MRSV](https://img.shields.io/badge/MSRV-1.82.0-orange?style=for-the-badge)](https://github.com/rust-lang/rust/releases/tag/1.82.0)
[![License](https://img.shields.io/badge/license-MPL_2.0-orange?style=for-the-badge&logo=mozilla)](https://github.com/nav-solutions/qc-traits/blob/main/LICENSE)

GNSS-Qc is the intermediate layer of the [NAV-Solutions framework](https://github.com/nav-solutions):
it gathers the GNSS and geodesy processing cores (synchronization, quality control,
navigation...) so that applications only have to feed data in and consume results.
[RINEX-Cli](https://github.com/nav-solutions/rinex-cli) is its reference application.

GNSS-Qc is licensed under the [Mozilla V2 Public](https://www.mozilla.org/en-US/MPL/2.0) license.

## Principles

GNSS-Qc is a **MISO** processor (Multiple Inputs, Single Output):

```text
 OBS receiver A ──┐                          ┌─ signals, coverage..
 OBS receiver B ──┤                          │
 NAV (agency) ────┼──► QcEngine ──► processors ──► one output stream ──► your application
 SP3 (agency) ────┤    synchronization       │                          (display, files,
 RTCM socket  ────┘    per satellite         └─ P.V.T solutions          database, network..)
 ANTEX (static)
```

- **Any input, any nature**: files (RINEX, CRINEX, SP3), streams (sockets, serial ports,
  decoders running in other threads), or anything implementing the `QcSource` trait.
  Any source may deliver any kind of data point (observations, ephemerides, precise states).
- **Pull based**: the application pulls results, the engine pulls just enough input data
  to produce them. Inputs are drained progressively, memory remains bounded
  (a few data points per satellite) whatever the session duration.
- **Synchronized**: inputs do not need to be aligned. Epochs are processed in chronological
  order, context data (ephemerides, precise orbits) is buffered per satellite around the
  epoch being processed.
- **Real-time or post-processed**: the same engine runs on files and on live streams,
  without async runtime.
- **Technology agnostic output**: a single stream of time tagged results. Rendering,
  storing or forwarding them is up to the application.

## Quick start

Process two daily observation files of the same receiver and 48h of ephemerides,
and verify that every satellite in sight is covered by a valid ephemeris:

```rust,no_run
use gnss_qc::prelude::*;

// 1. Context: files are indexed (receiver, agency..) and merged
let cfg = QcInputConfig {
    // these NAV files do not describe their agency: gather them with an alias
    nav_rinex_indexing: Some(QcIndexingConfig::Custom("NYA1-BRDC".to_string())),
    ..Default::default()
};

let mut products = QcInputProducts::default();

for file in [
    "data/CRNX/V3/NYA100NOR_S_20241270000_01D_30S_MO.crx.gz",
    "data/CRNX/V3/NYA100NOR_S_20241280000_01D_30S_MO.crx.gz",
    "data/NAV/V3/NYA100NOR_S_20241270000_01D_GN.rnx.gz",
    "data/NAV/V3/NYA100NOR_S_20241280000_01D_GN.rnx.gz",
] {
    products.load_gzip_file(&cfg, file).unwrap();
}

// 2. MISO engine: sources in, processors, one output stream
let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(QcEphemerisCoverageProcessor::default())
    .build()
    .unwrap();

// 3. Consume the results
for item in engine {
    let item = item.unwrap();
    if let QcOutput::EphemerisCoverage(coverage) = item.payload {
        if coverage.source.is_none() {
            println!("{} - {} is not covered", item.epoch, coverage.sv);
        }
    }
}
```

Navigation (P.V.T solutions) follows the same pattern, with the `QcNavigationProcessor`:
see the [navigation guide](docs/navigation/README.md).

## Documentation

| Topic | Contents |
|-------|----------|
| [Context definition](docs/context.md) | Input products, indexing (receivers, agencies, aliases), merging, user preferences, static data (ANTEX) |
| [MISO engine](docs/miso.md) | Sources, synchronization, lookahead, time windows, outputs and sinks, real-time operation |
| [Processors](docs/processors.md) | Built-in processors, writing your own processor and output type |
| [Navigation](docs/navigation/README.md) | P.V.T solutions with GNSS-RTK: orbital products (SP3 / NAV), configuration, performances |
| ↳ [SPP](docs/navigation/spp.md) | Single frequency code navigation |
| ↳ [CPP](docs/navigation/cpp.md) | Dual frequency (iono free) code navigation |
| ↳ [PPP](docs/navigation/ppp.md) | Float Precise Point Positioning (code + phase) |
| ↳ [RTK](docs/navigation/rtk.md) | Differential navigation: RTK-SPP, RTK-CPP, RTK-PPP |
| ↳ [CGGTTS](docs/navigation/cggtts.md) | Common view time transfer: P.V.T solutions post-fitted into CGGTTS tracks |
| [Ionospheric Pierce Points](docs/ipp.md) | Pierce points of the lines of sight in the ionosphere, over time |
| [Tests and data](docs/testing.md) | Test data, ANISE data, running the tests |

The API documentation is available on [docs.rs](https://docs.rs/gnss-qc/), where this guide
is also available (`gnss_qc::guide` module), with every code example verified at compile time.

## Examples

Runnable examples, based on the `data` submodule (run them from the repository root):

| Example | Demonstrates |
|---------|--------------|
| [context](examples/context.rs) | Loading files, indexing, merging, custom aliases |
| [miso](examples/miso.rs) | 48h streamed day after day, 48h of ephemerides, built-in + custom processors, custom output type, sink |
| [realtime](examples/realtime.rs) | A stream decoded in another thread, non blocking engine polling |
| [spp](examples/spp.rs) | Single Point Positioning |
| [cpp](examples/cpp.rs) | Code Precise Positioning |
| [ppp](examples/ppp.rs) | Precise Point Positioning |
| [rtk_ppp](examples/rtk_ppp.rs) | Differential navigation (RTK-PPP) |
| [cggtts](examples/cggtts.rs) | CGGTTS solutions, formatted into a CGGTTS file |
| [ipp](examples/ipp.rs) | Ionospheric Pierce Points time series |

```bash
git submodule update --init
cargo run --release --example miso
cargo run --release --features navigation --example ppp > solutions.csv
```

## Crate features

| Feature | Default | Description |
|---------|---------|-------------|
| `flate2` | ✓ | Gzip compressed files (`.gz`) |
| `sp3` | ✓ | SP3 precise products (sources, merging) |
| `navigation` | | P.V.T solutions, powered by [GNSS-RTK](https://github.com/nav-solutions/gnss-rtk) and [ANISE](https://github.com/nyx-space/anise). The heaviest option. |
| `cggtts` | | CGGTTS solutions (common view time transfer), powered by [CGGTTS](https://github.com/nav-solutions/cggtts) (implies `navigation`) |
| `ionex` | | Ionospheric Pierce Points, powered by [IONEX](https://github.com/nav-solutions/ionex) |
| `embed_ephem` | | Embeds the ANISE dataset at build time: no Internet access required at runtime (implies `navigation`) |
| `serde` | | Serialization of the configuration, indexing and output stream |
| `log` | | Debug traces (`$RUST_LOG`) |

Without `navigation`, GNSS-Qc provides context management, synchronization and the
processors that do not require navigation.

## Ecosystem

GNSS-Qc relies on:

- [Hifitime](https://github.com/nyx-space/hifitime): `Epoch`, `Duration` and `TimeScale` definitions
- [GNSS](https://github.com/nav-solutions/gnss): `Constellation` and `SV` definitions
- [GNSS-Qc-Traits](https://github.com/nav-solutions/qc-traits): behavior shared by the libraries of the framework (merging..)
- [RINEX](https://github.com/nav-solutions/rinex): RINEX parser, the most fundamental format
- [SP3](https://github.com/nav-solutions/sp3): precise products (`sp3` feature)
- [GNSS-RTK](https://github.com/nav-solutions/gnss-rtk) and [ANISE](https://github.com/nyx-space/anise): navigation (`navigation` feature)
- [CGGTTS](https://github.com/nav-solutions/cggtts): common view time transfer (`cggtts` feature)
- [IONEX](https://github.com/nav-solutions/ionex): ionosphere models and maps (`ionex` feature)
