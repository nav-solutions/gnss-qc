TEC maps (IONEX)
================

[← Meteo observations](meteo.md) · [README](../README.md) · [Ionospheric Pierce Points →](ipp.md)

IONEX files describe the state of the ionosphere: the vertical **Total Electron Content**
(TEC, in TECu: 10¹⁶ electrons/m²) over a grid of latitudes and longitudes (a region or the
whole Earth), at a given altitude, map after map (every 1 or 2 hours usually).

With the `ionex` feature, gnss-qc accepts them as inputs: the maps are synchronized like
every other context product, analyzed (statistics per region), interpolated in time to
synthesize new maps, and used by the [pierce points](ipp.md) (vertical TEC at each pierce point).

Contents:

- [Loading](#loading)
- [Synchronization and interpolation](#synchronization-and-interpolation)
- [TEC maps processor](#tec-maps-processor)
- [Synthesizing IONEX files](#synthesizing-ionex-files)
- [Results](#results)
- [Limitations](#limitations)

## Loading

IONEX files are loaded like any other file (`load_file`, `load_gzip_file`, or `load_ionex`
for an already parsed `IONEX`). They are indexed by agency (the `RUN BY` field of the
header, for example `AIUB` for CODE), see [indexing](context.md#indexing)
(`QcInputConfig::ionex_indexing`): agency, file name, or alias. Maps of the same
identity are gathered.

```rust,no_run
use gnss_qc::prelude::*;

let mut products = QcInputProducts::default();

products
    .load_gzip_file(&QcInputConfig::default(), "data/IONEX/V1/CKMG0090.21I.gz")
    .unwrap();

// agency:AIUB
for (agency, ionex) in products.ionex_iter() {
    println!("{} {} maps", agency, ionex.epoch_iter().count());
}
```

Each IONEX becomes a `QcIonexSource`, which delivers every node of every map
(`QcPayload::Tec`, `QcTecPoint`: latitude, longitude, altitude, TEC, RMS), map after map.
Any source declaring `QcSourceDescriptor::with_tec_maps()` may deliver TEC maps
(a real-time ionosphere model for example).

## Synchronization and interpolation

The engine gathers the nodes into maps (`QcTecMap`), per source and per epoch, in the
`QcTecStore` (`QcEpochContext::tec_maps`): the two maps that precede the epoch being
processed are kept, with every map ahead of it (lookahead).

| `QcTecStore` | Description |
|--------------|-------------|
| `tec(t, latitude, longitude, method)` | Vertical TEC at any instant and position, first source that can |
| `tec_from(source, t, latitude, longitude, altitude, method)` | Same, from a given source (altitude: layer of 3D maps) |
| `bracket(source, t)` | Maps surrounding `t` |
| `map(source, epoch)`, `maps(source)` | Buffered maps |

**Spatial interpolation**: bilinear interpolation of the 4 surrounding nodes (IONEX
specifications), nodes are returned as is; longitudes wrap around on worldwide maps.

**Temporal interpolation** (`QcTecInterpolation`), when `t` lies between two maps
E0 (T0) and E1 (T1):

| Strategy | Formula |
|----------|---------|
| `NearestMap` | E0 or E1, the nearest in time |
| `Linear` | E(t, φ, λ) = (T1 − t)/(T1 − T0) E0(φ, λ) + (t − T0)/(T1 − T0) E1(φ, λ) |
| `RotatedMaps` (default) | E(t, φ, λ) = (T1 − t)/(T1 − T0) E0(φ, λ + Ω(t − T0)) + (t − T0)/(T1 − T0) E1(φ, λ + Ω(t − T1)) |

with Ω = 360°/day. The ionosphere is strongly correlated with the Sun position: rotating
both maps around the Earth axis follows the Sun, it is the strategy recommended by the
IONEX specifications (Schaer et al., 1998). No extrapolation: `t` must lie between two maps.

## TEC maps processor

`QcTecProcessor` analyzes the maps, map after map, and may interpolate them in time:

| `QcTecConfig` | Default | Description |
|---------------|---------|-------------|
| `sources` | all | Sources (agencies) to process |
| `sampling` | `Native` | Output maps: `Native` (the maps of the products), `Period(dt)` (one map every `dt`), `Factor(k)` (native sampling divided by `k`) |
| `interpolation` | `RotatedMaps` | Temporal interpolation of the intermediate maps |
| `statistics` | true | Statistics of every map, per region (`QcOutput::TecMap`) |
| `regions` | global + 5 latitude bands | Regions of the statistics (`QcTecRegion`: name, latitude and longitude ranges) |
| `points` | false | Every node of every map (`QcOutput::TecPoint`) |

Intermediate maps are built on the grid of the preceding map, every node being
interpolated in time; native maps are streamed as is. Results are time tagged with the
map epoch, and flagged `interpolated` when the map is not a map of the product.

| `QcTecMapStatistics` | Description |
|----------------------|-------------|
| `source`, `region`, `interpolated` | Map identity |
| `nodes` | Number of nodes in the region |
| `mean_tecu`, `std_tecu`, `min_tecu`, `max_tecu` | Vertical TEC statistics (TECu) |
| `max_latitude_deg`, `max_longitude_deg` | Location of the maximal TEC |

```rust,no_run
use gnss_qc::prelude::*;

let mut products = QcInputProducts::default();

products
    .load_gzip_file(&QcInputConfig::default(), "data/IONEX/V1/CKMG0090.21I.gz")
    .unwrap();

let europe = QcTecRegion::new("Europe", (35.0, 70.0), (-10.0, 30.0));

let cfg = QcTecConfig::default()
    .with_sampling(Duration::from_seconds(900.0)) // 1h maps to 15' maps
    .with_regions(vec![QcTecRegion::global(), europe]);

let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(QcTecProcessor::new(cfg))
    .build()
    .unwrap();

for item in engine {
    let item = item.unwrap();
    if let QcOutput::TecMap(s) = item.payload {
        println!(
            "{} {} {:.1} TECu (max {:.1} at {:.1}°, {:.1}°)",
            item.epoch, s.region, s.mean_tecu, s.max_tecu, s.max_latitude_deg, s.max_longitude_deg
        );
    }
}
```

## Synthesizing IONEX files

gnss-qc is a MISO: it streams **points**, the application accumulates them. With
`points` enabled, every node of every map (native and interpolated) is streamed out as a
`QcTecMapPoint`; the application inserts them in an IONEX `Record` and formats a new file,
with the header of the original product (new temporal axis).

[examples/tec_maps.rs](../examples/tec_maps.rs): CKMG0080.09I (CODE, 2009-01-08, 13 maps,
one every 2 hours), interpolated in time ×2 (rotated maps): a new IONEX of 25 maps, one every
hour.

```rust,no_run
use gnss_qc::prelude::*;
use ionex::prelude::{Key, Record, IONEX, TEC};

let original = IONEX::from_gzip_file("data/IONEX/V1/CKMG0080.09I.gz").unwrap();

let mut products = QcInputProducts::default();
products
    .load_ionex(&QcInputConfig::default(), "CKMG0080.09I.gz", original.clone())
    .unwrap();

let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(QcTecProcessor::new(
        QcTecConfig::default().with_upsampling_factor(2.0).with_points(),
    ))
    .build()
    .unwrap();

// accumulate the nodes
let mut record = Record::default();

for item in engine {
    let item = item.unwrap();
    if let QcOutput::TecPoint(node) = item.payload {
        let p = node.point;
        record.insert(
            Key::from_decimal_degrees_km(item.epoch, p.latitude_deg, p.longitude_deg, p.altitude_km),
            TEC::from_tecu(p.tecu),
        );
    }
}

// new IONEX: header of the product, new temporal axis
let mut synthesized = original.with_record(record);
let epochs = synthesized.epoch_iter().collect::<Vec<_>>();
synthesized.header.epoch_of_first_map = epochs[0];
synthesized.header.epoch_of_last_map = *epochs.last().unwrap();
synthesized.header.number_of_maps = epochs.len() as u32;
synthesized.header.sampling_period = epochs[1] - epochs[0];

synthesized.to_file("CKMG0080_X2.09I").unwrap();
```

```bash
cargo run --release --features ionex --example tec_maps > tec_statistics.csv
```

## Pierce points

When TEC maps cover the session, the [pierce points](ipp.md) report the vertical TEC at
each pierce point (`QcIppOutput::vertical_tecu`, rotated maps interpolation), the slant
TEC along the line of sight (`slant_tecu()`) and the first order ionospheric delay
(`ionospheric_delay_m(frequency)`).

## Results

Verified by the test suite (`src/tests/ionex`), CODE global maps (2.5° × 5°, 5183 nodes):

- native maps are streamed as is (every node of CKMG0080.09I, 13 maps), the statistics of
  each region match a direct computation;
- maps interpolated in time (×2: 2h maps to 1h maps) are **identical to the resampling of
  the IONEX library** (`IONEX::temporally_resampled`), for the three strategies (within the
  0.001 TECu formatting resolution); 25 maps, 12 of them interpolated;
- periodic resampling of CKMG0090.21I (1h maps to 15' maps): 97 maps; with the linear
  strategy, the mean TEC of each interpolated map is the linear interpolation of the
  means of the surrounding maps.

CKMG0080.09I (solar minimum, January 2009), first map: global mean 10.6 TECu (σ 3.6),
maximum 25.5 TECu at (−10°, −145°), North high latitudes flat at 9.2 TECu.

## Limitations

- Temporal interpolation requires two consecutive maps of the same source: no
  extrapolation, nor interpolation between files of distinct periods.
- `RotatedMaps` on regional maps: nodes whose rotated coordinates leave the map are not
  interpolated (prefer `Linear` near the eastern and western borders).
- Statistics are computed over the nodes (not weighted by area).
- Differential code biases (DCB) of the IONEX files are not used yet.

Next: [Ionospheric Pierce Points →](ipp.md)
