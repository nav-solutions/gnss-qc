Ionospheric Pierce Points
=========================

[← Processors](processors.md) · [README](../README.md) · [Navigation →](navigation/README.md)

The `ionex` feature resolves the **Ionospheric Pierce Points** (IPP) of a receiver:
the points where the lines of sight (receiver to satellites in sight) pierce the
ionosphere, over time. It relies on the single layer model of the
[IONEX library](https://github.com/nav-solutions/ionex): the ionosphere is modeled as a thin
shell at a fixed altitude above a spherical Earth, as in the IONEX ionosphere maps.

```toml
gnss-qc = { version = "..", features = ["ionex"] }
```

The navigation feature is not required.

## Principle

```text
 OBS (receiver) ──┐                      ┌─ satellites in sight (signals of this epoch)
 NAV ─────────────┼──► QcEngine ──► QcIppProcessor ─┤
 SP3 ─────────────┘                      ├─ satellite position: SP3 (interpolated) or NAV (Kepler)
                                         ├─ line of sight: elevation, azimuth
                                         └─ IONEX single layer model ──► IPP (lat, long, obliquity)
```

At each epoch, for each receiver and each satellite in sight:

1. the satellite position is resolved from the synchronized context, according to the
   orbital products preference (`QcOrbitPreference`): precise products (Lagrange interpolation
   of the SP3) or broadcast ephemerides (Keplerian solving), see
   [orbital products](navigation/README.md#orbital-products);
2. the line of sight (elevation, azimuth) is computed from the receiver position;
3. below the elevation mask, the satellite is discarded;
4. the pierce point is determined on the shell: latitude, longitude, and the obliquity factor
   `F = 1 / cos(z')` that converts vertical TEC at the pierce point to slant TEC along the
   line of sight (`STEC = F x VTEC`).

The receiver position is the reference position of its signal source (antenna reference
point of the RINEX header), or a position defined in the configuration.

## Configuration

`QcIppConfig`:

| Field | Default | Description |
|-------|---------|-------------|
| `orbits` | `Auto` | Orbital products preference (precise products first) |
| `elevation_mask_deg` | 10° | Elevation mask |
| `shell_altitude_km` | 450 km | Altitude of the ionospheric shell (IGS ionosphere maps) |
| `earth_radius_km` | 6371 km | Radius of the spherical Earth (IGS ionosphere maps) |
| `receivers` | all | Receivers to process |
| `constellations` | all | Constellations to process |
| `receiver_position_ecef_m` | source reference position | Receiver position (ECEF m) |

To compare with an IONEX map, use its shell altitude and base radius (IONEX header,
`HGT1 / HGT2 / DHGT` and `BASE RADIUS`).

## Example

[examples/ipp.rs](../examples/ipp.rs)

```rust,no_run
use gnss_qc::prelude::*;

let mut products = QcInputProducts::default();

for file in [
    "data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz",
    "data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz",
    "data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz",
] {
    products.load_gzip_file(&QcInputConfig::default(), file).unwrap();
}

let ipp = QcIppProcessor::new(QcIppConfig {
    orbits: QcOrbitPreference::Auto,
    elevation_mask_deg: 10.0,
    shell_altitude_km: 450.0,
    ..Default::default()
});

let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(ipp)
    .build()
    .unwrap();

for item in engine {
    let item = item.unwrap();
    if let QcOutput::IPP(ipp) = item.payload {
        println!(
            "{} {} ({:?}) elev={:.1}° IPP=({:.3}°, {:.3}°) F={:.3}",
            item.epoch,
            ipp.sv,
            ipp.orbit,
            ipp.elevation_deg,
            ipp.latitude_deg,
            ipp.longitude_deg,
            ipp.obliquity_factor
        );
    }
}
```

```bash
cargo run --release --features ionex --example ipp > ipp.csv
```

```text
epoch,sv,orbit,elevation_deg,azimuth_deg,ipp_lat_deg,ipp_long_deg,obliquity
2020-06-25T00:00:00 GPST,R01,Precise,83.616,133.459,55.2016,8.9945,1.0054
2020-06-25T00:00:00 GPST,E01,Precise,16.147,36.652,62.9738,21.7275,2.2642
2020-06-25T00:00:00 GPST,R02,Precise,28.177,310.166,59.2795,-1.1458,1.7620
```

## Outputs

`QcOutput::IPP(QcIppOutput)`, one per line of sight and per epoch:

| Field | Description |
|-------|-------------|
| `receiver` | Receiver |
| `sv` | Satellite |
| `orbit` | Product the satellite position was resolved from (`Precise`, `Broadcast`) |
| `elevation_deg`, `azimuth_deg` | Line of sight, at the receiver |
| `latitude_deg`, `longitude_deg` | Pierce point |
| `altitude_km` | Shell altitude |
| `obliquity_factor` | Slant factor: `STEC = factor x VTEC` |
| `vertical_tecu` | Vertical TEC at the pierce point (TECu), when TEC maps cover it, see below |

`slant_tecu()` (vertical TEC × obliquity factor) and `ionospheric_delay_m(frequency)`
(first order delay, 40.3 STEC / f²) are available when the vertical TEC is known.

## TEC maps

When TEC maps (IONEX files) are loaded with the observations, the vertical TEC at each pierce
point is interpolated from the maps (bilinear in space, rotated maps in time), see
[TEC maps](ionex.md). Verified by the test suite with synthetic worldwide maps
(`src/tests/ipp.rs`).

## Orbital products

| Constellation | Precise products (SP3) | Broadcast ephemerides (NAV) |
|---------------|------------------------|-----------------------------|
| GPS, Galileo, QZSS | ✓ (if in the products) | ✓ |
| BeiDou | ✓ (if in the products) | ✓ (MEO, IGSO, GEO) |
| Glonass | ✓ (if in the products) | not supported |

With `Auto`, each satellite uses the precise products whenever they cover it, the broadcast
ephemerides otherwise. Both give the same pierce points: the line of sight is not sensitive
to meter level orbital differences (< 0.001°).

## Results

ESBC00DNK, 2020-06-25 00:00-01:00, NAV + GRG SP3 (verified by the test suite,
`src/tests/ipp.rs` and `src/tests/navigation/ipp.rs`):

- ~2800 pierce points per hour and per product (GPS, Galileo, plus Glonass with precise
  products or BeiDou with broadcast ephemerides), 10° elevation mask;
- precise and broadcast orbits give the same pierce points (< 0.001°);
- the lines of sight match the elevations and azimuths of the navigation solver (< 0.01°);
- the pierce points remain within ~15° of the station (450 km shell, 10° mask), and move
  continuously over time.

## Limitations

- The satellite position is evaluated at the sampling instant (the transmission instant is
  ~70 ms earlier): a few meters on the pierce point, negligible.
- The receiver altitude is neglected by the single layer model.
- TEC values (IONEX maps) at the pierce points are not provided yet.
