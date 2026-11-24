Orbit residuals
===============

[← Observation residuals](observations.md) · [Residual analysis](README.md) · [README](../../README.md) · [Clock residuals →](clocks.md)

`QcOrbitResidualProcessor` differentiates the satellite states described by two orbital
products: for each satellite described by both products, at each epoch (or at the requested
sampling), the states are resolved from both products and differentiated: **A − B**, in ECEF
and in the radial / along track / cross track frame of A, plus the clock residual.

| Comparison | Configuration | Typical residuals |
|------------|---------------|-------------------|
| Precise − precise (two agencies) | `QcOrbitResidualConfig::precise(a, b)` | cm (orbits), ns (clocks: clock datum of each agency) |
| Broadcast − precise | `QcOrbitResidualConfig::broadcast_precise()` | m |
| Broadcast − broadcast (two sources) | `QcOrbitResidualConfig::broadcast(a, b)` | 0 for identical ephemerides |

## Principle

```text
 SP3 (agency A) ──┐                                 ┌─ product A: state of SV at t
 SP3 (agency B) ──┤                                 │   precise: Lagrange interpolation (order 10)
 NAV (source A) ──┼──► QcEngine ──► QcOrbitResidualProcessor ─┤   broadcast: Keplerian solving
 NAV (source B) ──┘                                 ├─ product B: state of SV at t
                                                    └─► A − B: ECEF, radial / along / cross, clock
```

Each side is a `QcOrbitProduct`:

| `QcOrbitProduct` | States |
|------------------|--------|
| `Precise(Some(agency))` | precise products of this agency |
| `Precise(None)` | precise products of any agency |
| `Broadcast(Some(source))` | broadcast ephemerides of this source (file stem or alias) |
| `Broadcast(None)` | broadcast ephemerides of any source |

The radial / along track / cross track frame is defined by the state of A
(radial: position, cross track: angular momentum, along track completes the frame).
A comparison of a product with itself (same source on both sides) is skipped.

Without observation, the session is *context only*: the engine iterates over the epochs of
the products themselves (precise products samples, broadcast times of clock), see
[MISO engine](../miso.md). With observations, residuals follow the receivers epochs: use
`sampling` to decimate.

## Configuration

`QcOrbitResidualConfig`:

| Field | Default | Description |
|-------|---------|-------------|
| `lhs` | `Broadcast(None)` | Product A |
| `rhs` | `Precise(None)` | Product B |
| `sampling` | every epoch | Minimal interval between two evaluations |
| `constellations` | all | Constellations to process |
| `max_broadcast_age` | validity period of the constellation | Broadcast ephemerides: maximal distance between the evaluation instant and the time of clock |

Helpers: `precise(a, b)`, `broadcast_precise()` (= default), `broadcast(a, b)`,
`.with_sampling()`, `.with_constellations()`, `.with_max_broadcast_age()`.

Single station navigation files also contain distant ephemerides of satellites out of sight
(tracked once, hours ago): a broadcast − precise comparison then characterizes extrapolated
orbits. Restricting to recent ephemerides (`with_max_broadcast_age(1 h)`) characterizes the
broadcast ephemerides as a receiver uses them.

## Example

[examples/orbit_residuals.rs](../../examples/orbit_residuals.rs)

```rust,no_run
use gnss_qc::prelude::*;

let mut products = QcInputProducts::default();

for file in [
    "data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz",
    "data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz",
] {
    products.load_gzip_file(&QcInputConfig::default(), file).unwrap();
}

// broadcast ephemerides - precise products, every 15', recent ephemerides only
let cfg = QcOrbitResidualConfig::broadcast_precise()
    .with_sampling(Duration::from_seconds(900.0))
    .with_max_broadcast_age(Duration::from_hours(1.0));

let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(QcOrbitResidualProcessor::new(cfg))
    .build()
    .unwrap();

for item in engine {
    let item = item.unwrap();
    if let QcOutput::OrbitResidual(r) = item.payload {
        println!(
            "{} {} R={:.3} A={:.3} C={:.3} |dr|={:.3} m",
            item.epoch, r.sv, r.radial_m, r.along_track_m, r.cross_track_m, r.norm_m()
        );
    }
}
```

```bash
cargo run --release --example orbit_residuals > residuals.csv
```

```text
epoch,sv,lhs,lhs_kind,rhs,rhs_kind,radial_m,along_m,cross_m,norm_m,clock_ns
2020-06-25T00:00:00 GPST,G01,agency:GRGS,Precise,agency:IAC,Precise,0.010,-0.041,-0.023,0.048,1.865
2020-06-25T00:00:00 GPST,R01,agency:GRGS,Precise,agency:IAC,Precise,-0.013,0.010,-0.014,0.022,-3.531
2020-06-25T00:00:00 GPST,G02,ESBC00DNK_R_20201770000_01D_MN,Broadcast,agency:GRGS,Precise,0.038,2.851,1.940,3.449,1.296
```

## Outputs

`QcOutput::OrbitResidual(QcOrbitResidual)`, one per satellite and per evaluation:

| Field | Description |
|-------|-------------|
| `sv` | Satellite |
| `lhs_source`, `rhs_source` | Products A and B (agency or NAV source) |
| `lhs_kind`, `rhs_kind` | `Precise` or `Broadcast` |
| `lhs_toc`, `rhs_toc` | Time of clock of the broadcast ephemeris that was used (broadcast only) |
| `ecef_m` | A − B, ECEF (m) |
| `radial_m`, `along_track_m`, `cross_track_m` | A − B in the orbital frame of A (m) |
| `clock_s` | Clock residual A − B (s), when both products provide a clock |
| `norm_m()` | 3D residual (m) |

## Results

2020-06-25, verified by the test suite (`src/tests/residuals`). RMS per component.

**Precise − precise: GRG − IAC** (15', 7200 residuals):

| Constellation | Radial | Along | Cross | 3D RMS | 3D max | Clock RMS |
|---------------|--------|-------|-------|--------|--------|-----------|
| GPS | 1.9 cm | 2.3 cm | 2.0 cm | 3.5 cm | 10.1 cm | 1.5 ns |
| Galileo | 2.6 cm | 2.1 cm | 2.0 cm | 3.9 cm | 9.5 cm | 2.6 ns |
| Glonass | 2.0 cm | 3.5 cm | 3.4 cm | 5.3 cm | 14.2 cm | 2.8 ns |

The clock residuals (ns) are dominated by the clock datum of each analysis center (a common
offset per epoch), not by the satellite clocks themselves.

**Broadcast − precise: NAV (ESBC00DNK) − GRG** (15', ephemerides younger than 1 hour):

| Constellation | Radial | Along | Cross | 3D RMS | 3D max | Clock RMS |
|---------------|--------|-------|-------|--------|--------|-----------|
| GPS | 1.06 m | 0.78 m | 0.49 m | 1.41 m | 3.95 m | 2.2 ns |
| Galileo (*) | 0.87 m | 0.45 m | 0.32 m | 1.03 m | 4.7 m | 0.7 ns |

(*) without E14 and E18: these two satellites (eccentric orbits after their launch anomaly,
testing status in 2020) reach 40 m.

The broadcast orbits refer to the antenna phase center, the precise products to the center
of mass: no antenna offset is applied, the residuals include this offset (mostly radial).

**Broadcast − broadcast: NAV ESBC00DNK − NAV MOJN00DNK** (two stations, same day):

- identical ephemerides (same time of clock) give exactly the same orbit (0 m) and, for GPS,
  the same clock: both receivers decoded the same messages;
- different times of clock reveal the ephemeris update policy of each receiver (`lhs_toc`,
  `rhs_toc`);
- **Galileo**: same time of clock, same orbit, but clock residuals of a few ns (E11:
  −1.8 ns): one file reports the I/NAV clock (E1-E5b reference), the other the F/NAV clock
  (E1-E5a reference). Mixing both message types for the same satellite is a classical
  pitfall of merged navigation files, that this comparison reveals immediately.

## Limitations

- No antenna phase center correction between broadcast (APC) and precise (CoM) states.
- Broadcast clocks: polynomial only, without the periodic relativistic correction (excluded
  from the precise clocks as well) and without group delay (TGD / BGD).
- Glonass broadcast ephemerides are not supported (precise products only), as in the
  [orbital products](../navigation/README.md#orbital-products) of the navigation.
- A − B for one pair of products; run several processors for several pairs.

Next: [Clock residuals →](clocks.md)
