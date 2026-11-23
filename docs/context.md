Context definition
==================

[← README](../README.md) · [MISO engine →](miso.md)

A GNSS-Qc *context* is the set of data a session works with. It combines:

- **temporal data**, delivered by *sources* as time tagged data points
  (signal observations, radio ephemerides, precise orbits..), see the [MISO engine](miso.md);
- **static data**, valid for the whole session (antenna calibrations, ANTEX).

Files are first gathered in a `QcInputProducts` table, which identifies (*indexes*) and
merges them, then converts them into engine sources.

## Indexing

Every product is identified by a `QcIndexing`. The identity defines what is merged
and how the data points are presented to the processors:

| `QcIndexing` | Typical use |
|--------------|-------------|
| `Receiver(String)` | Signal observations: one entry per GNSS receiver. Several receivers can be processed at once (RTK base and rover) |
| `Antenna(String)` | Signal observations, identified by antenna |
| `Operator(String)` | Observations identified by operator ("observer" in RINEX) |
| `Agency(String)` | Navigation and precise products, identified by their publisher: products of different agencies can be compared (residual analysis) |
| `Custom(String)` | User defined alias |

### Automatic indexing

When no preference applies (or when the preferred information is missing from the file),
the identity is determined automatically, and this is infallible:

| Product | Identity |
|---------|----------|
| Observation RINEX / CRINEX | Receiver `"MODEL #SERIAL"`, otherwise antenna model, otherwise operator, otherwise file name |
| Navigation, Meteo, Clock RINEX | Agency, otherwise file name |
| SP3 | Agency (always described) |

The receiver serial number is part of its identity: two stations using the same receiver
model (typical RTK base and rover) remain two different signal sources.

```text
receiver:SEPT POLARX5 #3022663     (MOJN00DNK)
receiver:SEPT POLARX5 #3047937     (ESBC00DNK)
agency:GRGS                        (GRG0MGXFIN SP3)
ESBC00DNK_R_20201770000_01D_MN     (NAV file without agency: file name)
```

### Preferences

`QcInputConfig` defines the indexing method per RINEX type
(`QcIndexingConfig::{Receiver, Antenna, Operator, Agency, Custom(alias)}`).
Defaults: receiver for observations, agency for the others.

A custom alias gathers files that would be indexed differently. Here, four navigation files
of the NYA1 station (GPS and Galileo, two days) that do not describe their agency:

```rust,no_run
use gnss_qc::prelude::*;

let cfg = QcInputConfig {
    nav_rinex_indexing: Some(QcIndexingConfig::Custom("NYA1-BRDC".to_string())),
    ..Default::default()
};

let mut products = QcInputProducts::default();

for file in [
    "data/NAV/V3/NYA100NOR_S_20241270000_01D_GN.rnx.gz",
    "data/NAV/V3/NYA100NOR_S_20241270000_01D_EN.rnx.gz",
    "data/NAV/V3/NYA100NOR_S_20241280000_01D_GN.rnx.gz",
    "data/NAV/V3/NYA100NOR_S_20241280000_01D_EN.rnx.gz",
] {
    products.load_gzip_file(&cfg, file).unwrap();
}

// a single source: 48h of GPS and Galileo ephemerides
assert_eq!(products.nav_rinex_iter().count(), 1);
```

## Loading and merging

| Method | Description |
|--------|-------------|
| `load_file(cfg, path)` | Any supported file: RINEX formats are attempted first, then SP3 |
| `load_gzip_file(cfg, path)` | Same, gzip compressed (`flate2` feature) |
| `load_rinex(cfg, path, rinex)` | An already parsed `Rinex` |
| `load_sp3(sp3)` | An already parsed `SP3` (`sp3` feature) |

Files sharing the same identity and type are **merged**. Two daily observation files of
the same receiver become a single 48h signal source:

```rust,no_run
use gnss_qc::prelude::*;

let mut products = QcInputProducts::default();
let cfg = QcInputConfig::default();

products
    .load_gzip_file(&cfg, "data/CRNX/V3/NYA100NOR_S_20241270000_01D_30S_MO.crx.gz")
    .unwrap();
products
    .load_gzip_file(&cfg, "data/CRNX/V3/NYA100NOR_S_20241280000_01D_30S_MO.crx.gz")
    .unwrap();

assert_eq!(products.obs_rinex_iter().count(), 1);
```

Merging requires both files to be held in memory. For long sessions, chaining sources
(one file opened at a time) is preferable: see [streaming files](miso.md#streaming-files).

The table can be inspected before processing:

```rust,no_run
use gnss_qc::prelude::*;

let products = QcInputProducts::default();

for (indexing, rinex) in products.obs_rinex_iter() {
    println!("{} - reference position {:?}", indexing, rinex.header.rx_position);
}

for (indexing, rinex) in products.nav_rinex_iter() {
    println!("{} - {} frames", indexing, rinex.record.as_nav().unwrap().len());
}

for (indexing, sp3) in products.sp3_iter() {
    println!("{} - {} states", indexing, sp3.data.len());
}
```

## From products to sources

The engine does not work with files but with sources. `QcEngineBuilder::with_products`
converts the table (`QcInputProducts::into_sources`):

| Product | Engine source |
|---------|---------------|
| Observation RINEX / CRINEX | `QcRinexObsSource`: signal observations and receiver clock, epoch after epoch. The antenna reference point (header position + antenna eccentricities) is attached to the source |
| Navigation RINEX | `QcRinexNavSource`: ephemeris frames, chronological order (ToC) |
| SP3 | `QcSp3Source`: precise states (position, clock), chronological order |
| ANTEX | Static data (`QcStatics`), not a source |
| Meteo, Clock RINEX | Not converted yet |

Sources drain the records they were built from: memory is released as the engine progresses.

## Static data

Static data is valid for the whole session, it is not synchronized but available to every
processor at every epoch (`QcEpochContext::statics`). ANTEX databases loaded in the table
are converted automatically, they can also be provided directly:

```rust,no_run
use gnss_qc::prelude::*;
use rinex::prelude::Rinex;

let antex = Rinex::from_file("igs20.atx").unwrap();

let builder = QcEngine::<QcOutput>::builder()
    .with_antex(QcIndexing::Custom("igs20".to_string()), antex);
```

Next: [MISO engine →](miso.md)
