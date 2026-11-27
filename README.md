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
 CLK (agency) ────┤    per satellite         └─ P.V.T solutions          database, network..)
 MET (station) ───┤
 NTRIP clients ───┘  (RTCM streams: rover, base, ephemerides..)
 ANTEX (static)
```

- **Any input, any nature**: files (Observation, Navigation, Clock and Meteo RINEX, CRINEX,
  SP3, IONEX), streams (NTRIP clients, RTCM, sockets, serial ports, decoders running in other threads), or anything
  implementing the `QcSource` trait. Any source may deliver any kind of data point
  (observations, ephemerides, precise states, precise clocks, meteo observations, TEC maps).
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
| [Precise clocks](docs/clocks.md) | Clock RINEX: satellite and station clocks, synchronization, satellite clocks of the navigation (Clock RINEX > SP3 > broadcast) |
| [Meteo observations](docs/meteo.md) | Meteo RINEX: sensor observations, synchronization, stream |
| [RTCM streams and NTRIP clients](docs/ntrip.md) | Real-time inputs: as many NTRIP clients as needed (rover, base, ephemerides), RTCM 3 decoding, indexing and roles, RTK in real-time |
| [Processors](docs/processors.md) | Built-in processors, writing your own processor and output type |
| [Navigation](docs/navigation/README.md) | P.V.T solutions with GNSS-RTK: orbital products (SP3 / NAV), satellite clocks (CLK / SP3 / NAV), configuration, performances |
| ↳ [SPP](docs/navigation/spp.md) | Single frequency code navigation |
| ↳ [CPP](docs/navigation/cpp.md) | Dual frequency (iono free) code navigation |
| ↳ [PPP](docs/navigation/ppp.md) | Float Precise Point Positioning (code + phase) |
| ↳ [RTK](docs/navigation/rtk.md) | Differential navigation: RTK-SPP, RTK-CPP, RTK-PPP |
| ↳ [CGGTTS](docs/navigation/cggtts.md) | Common view time transfer: P.V.T solutions post-fitted into CGGTTS tracks |
| [TEC maps (IONEX)](docs/ionex.md) | IONEX inputs: TEC maps statistics per region, temporal interpolation, synthesis of new IONEX files |
| [Ionospheric Pierce Points](docs/ipp.md) | Pierce points of the lines of sight in the ionosphere, vertical TEC from the maps |
| [TEC from signal observations](docs/obs_tec.md) | Slant and vertical TEC estimated from dual frequency observations (leveled phase), at the pierce points of a network of receivers: the raw material of TEC maps |
| [Real-time TEC maps from NTRIP streams](docs/ntrip_tec.md) | N NTRIP clients (a network of reference stations), real-time TEC estimation, TEC maps published as IONEX (uncalibrated) |
| [Signal combinations](docs/combinations.md) | Geometry Free, Ionosphere Free and code multipath, for every pair of frequencies, phase arcs |
| [Residual analysis](docs/residuals/README.md) | A − B analyses of standard products, available by default |
| ↳ [Observation residuals](docs/residuals/observations.md) | Receiver A − receiver B: relative clock behavior, receiver artifacts |
| ↳ [Orbit residuals](docs/residuals/orbits.md) | Agency A − agency B, broadcast − precise, NAV − NAV |
| ↳ [Clock residuals](docs/residuals/clocks.md) | Clock products of two agencies (Clock RINEX, SP3), precise − broadcast |
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
| [ntrip](examples/ntrip.rs) | NTRIP clients (one per argument: rover, base, ephemerides), real-time P.V.T solutions |
| [spp](examples/spp.rs) | Single Point Positioning |
| [cpp](examples/cpp.rs) | Code Precise Positioning |
| [ppp](examples/ppp.rs) | Precise Point Positioning |
| [rtk_ppp](examples/rtk_ppp.rs) | Differential navigation (RTK-PPP) |
| [cggtts](examples/cggtts.rs) | CGGTTS solutions, formatted into a CGGTTS file |
| [ipp](examples/ipp.rs) | Ionospheric Pierce Points time series |
| [obs_tec](examples/obs_tec.rs) | TEC estimated from the observations of two receivers, at their pierce points (CSV) |
| [ntrip_tec](examples/ntrip_tec/main.rs) | Real-time TEC maps of a network: one NTRIP client per station, IONEX file of the day updated map after map |
| [tec_maps](examples/tec_maps.rs) | TEC maps statistics, interpolation ×2, synthesis of a new IONEX file |
| [clock_residuals](examples/clock_residuals.rs) | GRG (Clock RINEX) − IAC (SP3) clock residuals |
| [precise_clocks](examples/precise_clocks.rs) | Satellite and station clocks of a Clock RINEX (CODE MGEX) |
| [meteo](examples/meteo.rs) | Meteo observations time series (POTS00DEU) |
| [combinations](examples/combinations.rs) | GF, IF and code multipath of a geodetic receiver (ESBC00DNK) |
| [observation_residuals](examples/observation_residuals.rs) | PolaRx5 − GNSS-SDR observation residuals (common antenna) |
| [orbit_residuals](examples/orbit_residuals.rs) | GRG − IAC and NAV − GRG orbit residuals |

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
| `ionex` | | IONEX inputs (TEC maps analysis, temporal interpolation), Ionospheric Pierce Points and TEC estimated from the observations, powered by [IONEX](https://github.com/nav-solutions/ionex) |
| `rtcm` | | RTCM 3 decoding: MSM (GPS, Glonass, Galileo, SBAS, QZSS, BeiDou) and legacy observations, radio ephemerides, station coordinates and descriptors as inputs, powered by [rtcm-rs](https://github.com/martinhakansson/rtcm-rs), see [RTCM streams](docs/ntrip.md) |
| `ntrip` | | NTRIP clients as real-time inputs (as many as needed), powered by [ntrip-client](https://github.com/nav-solutions/ntrip-client) and [Tokio](https://tokio.rs) (implies `rtcm`) |
| `embed_ephem` | | Embeds the ANISE dataset at build time: no Internet access required at runtime (implies `navigation`) |
| `serde` | | Serialization and deserialization of the configurations, indexing, data points and output stream, see [Serde support](docs/context.md#serde-support) |
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
- [NTRIP client](https://github.com/nav-solutions/ntrip-client) and [rtcm-rs](https://github.com/martinhakansson/rtcm-rs): real-time RTCM streams (`ntrip` and `rtcm` features)
