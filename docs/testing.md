Tests and data
==============

[← README](../README.md)

## Test data

Tests and examples use the [data](https://github.com/rtk-rs/data) submodule:

```bash
git submodule update --init
```

| Dataset | Used for |
|---------|----------|
| `OBS/V3/ACOR00ESP_R_20213550000_01D_30S_MO.rnx` | single observation file, real-time replay |
| `CRNX/V3/NYA100NOR_S_2024127*`, `2024128*` | 48h of the same receiver (merged, chained) |
| `NAV/V3/NYA100NOR_S_2024127*`, `2024128*` (GN, EN) | 48h of GPS + Galileo ephemerides |
| `CRNX/V3/ESBC00DNK_R_20201770000*`, `MOJN00DNK_R_20201770000*` | navigation, RTK (two stations, same receiver model) |
| `NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz` | multi-GNSS broadcast ephemerides |
| `SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz` | precise orbits and clocks (GPS, Glonass, Galileo) |

## ANISE data

Navigation tests require the ANISE dataset: a JPL planetary ephemeris (`de440s.bsp`, or any
planetary SPK covering the data, like `de421.bsp`) and `pck11.pca`. They are loaded from
`$GNSS_QC_ANISE_DIR` (or `<repo>/.cache`), and downloaded by ANISE otherwise
(Internet access required).

```bash
export GNSS_QC_ANISE_DIR=/path/to/anise-data
```

The examples use `$GNSS_QC_ANISE_DIR` when defined, `MetaAlmanac::latest()` otherwise.

## Running the tests

```bash
cargo test                       # context, MISO engine, synchronization (data submodule)
cargo test --features navigation # + navigation scenarios (ANISE data)
cargo test --features cggtts,ionex # + CGGTTS and Ionospheric Pierce Points
cargo test --features serde      # + serialization round trips
cargo test --doc                 # documentation examples (compiled, not run)
```

Test suites:

| Module | Verifies |
|--------|----------|
| `tests::engine` | engine invariants on synthetic data: misaligned receivers, coverage, late data, real-time channel |
| `tests::miso` | single OBS / CRINEX / NAV files, 48h combinations (merged, chained), coverage, RTK pair, real-time replay |
| `tests::navigation` | Keplerian solving against SP3, SPP, CPP, PPP, RTK-CPP, RTK-PPP scenarios, precise products coverage loss, satellite clocks preferences (Clock RINEX, SP3, broadcast) |
| `tests::navigation::cggtts` | CGGTTS (SPP, CPP, PPP): schedule, REFSYS / REFSV / MDTR against independent references, CGGTTS file round trip, RTK rejection |
| `tests::clock` | Clock RINEX and Meteo RINEX sources, GRG Clock RINEX against GRG SP3 clocks, 15' clock interpolation error, precise clocks and meteo streams, meteo context |
| `tests::input` | SP3 indexing preferences (agency, file name, custom alias, fallbacks), in the engine |
| `tests::serde` | `serde` feature: Serialize / Deserialize of every public type, JSON round trips of configurations, data points and real output streams |
| `tests::residuals` | observation residuals against `Rinex::observations_substract`, receiver clock analysis (PolaRx5 − GNSS-SDR), misaligned receivers, orbit residuals GRG − IAC, NAV − SP3, NAV − NAV |
| `tests::ipp`, `tests::navigation::ipp` | pierce points: precise versus broadcast orbits, geometry, time series, lines of sight against the navigation solver |

Release builds are much faster to run the data based tests:

```bash
cargo test --release --features cggtts,ionex
```
