Clock residuals
===============

[← Orbit residuals](orbits.md) · [Residual analysis](README.md) · [README](../../README.md)

`QcClockResidualProcessor` differentiates the clocks described by two clock products:
for each clock described by both products (satellite clocks, ground station clocks), at each
epoch (or at the requested sampling): **A − B**. Each side may be:

| `QcClockProduct` | Clocks | Interpolation |
|------------------|--------|---------------|
| `Clock(Some(agency))`, `Clock(None)` | Clock RINEX: satellites (`AS`) and ground stations (`AR`) | linear |
| `PreciseState(Some(agency))`, `PreciseState(None)` | SP3 clock states: satellites | linear |
| `Broadcast(Some(source))`, `Broadcast(None)` | radio messages (NAV): satellites | clock polynomial |

`None` designates the first product (sorted by name) that describes the clock. A product
compared to itself (same source, same kind) is skipped.

| Comparison | Configuration | Reveals |
|------------|---------------|---------|
| Two analysis centers, Clock RINEX | `QcClockResidualConfig::clock_products(a, b)` | consistency of the clock solutions (0.1 ns level) |
| Clock RINEX − SP3 of another center | `QcClockResidualConfig::new(Clock(a), PreciseState(b))` | same |
| Clock RINEX − SP3 of the same center | `QcClockResidualConfig::default()` | interpolation error of the SP3 clock states |
| Precise − broadcast | `QcClockResidualConfig::new(Clock(None), Broadcast(None))` | quality of the broadcast clocks |

## Clock datum

Each analysis center estimates its clocks relative to its own reference (a station clock,
or an ensemble): **the raw residuals of two centers share a common offset at each epoch**,
of a few nanoseconds, that says nothing about the quality of the clocks. This datum
difference is estimated at each epoch (median of the satellite residuals, robust to
outliers) and reported with each residual (`datum_s`); `aligned_s()` is the residual once
the datum is removed:

| `QcClockAlignment` | Datum |
|--------------------|-------|
| `None` | not estimated (`datum_s` is `None`) |
| `Epoch` | one datum per epoch, all satellites |
| `Constellation` (default) | one datum per epoch and per constellation (the inter system biases differ between centers). Station clocks use the GPS datum |

At least `min_datum_satellites` satellites (3 by default) are required to estimate a datum.

## Configuration

`QcClockResidualConfig`:

| Field | Default | Description |
|-------|---------|-------------|
| `lhs`, `rhs` | `Clock(None)`, `PreciseState(None)` | Products A and B |
| `satellites` | true | Satellite clocks |
| `stations` | true | Ground station clocks (Clock RINEX only) |
| `sampling` | every epoch | Minimal interval between two evaluations |
| `constellations` | all | Constellations |
| `alignment` | `Constellation` | Clock datum estimation |
| `min_datum_satellites` | 3 | Minimal number of satellites of a datum |

In a context only session, the epochs are the epochs of the products: every 30" with
Clock RINEX. Compared to SP3 clock states, prefer the sampling of the SP3 (15' for
example): in between, the SP3 clocks are interpolated (see [results](#results)).

## Example

[examples/clock_residuals.rs](../../examples/clock_residuals.rs)

```rust,no_run
use gnss_qc::prelude::*;

let mut products = QcInputProducts::default();

for file in [
    "data/CLK/V3/GRG0MGXFIN_20201770000_01D_30S_CLK.CLK.gz",
    "data/SP3/D/Sta21114.sp3.gz",
] {
    products.load_gzip_file(&QcInputConfig::default(), file).unwrap();
}

// GRG (Clock RINEX) - IAC (SP3), every 15'
let cfg = QcClockResidualConfig::new(
    QcClockProduct::Clock(Some(QcIndexing::Agency("GRG".to_string()))),
    QcClockProduct::PreciseState(Some(QcIndexing::Agency("IAC".to_string()))),
)
.with_sampling(Duration::from_seconds(900.0));

// two Clock RINEX:
// QcClockResidualConfig::clock_products(Agency("GRG"), Agency("JPL"))

let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(QcClockResidualProcessor::new(cfg))
    .build()
    .unwrap();

for item in engine {
    let item = item.unwrap();
    if let QcOutput::ClockResidual(r) = item.payload {
        println!(
            "{} {} raw {:.3} ns, aligned {:.3} ns",
            item.epoch,
            r.clock,
            r.residual_s * 1.0E9,
            r.aligned_s().unwrap_or(f64::NAN) * 1.0E9,
        );
    }
}
```

```text
epoch,clock,lhs,rhs,residual_ns,datum_ns,aligned_ns
2020-06-25T00:00:00 GPST,G01,agency:GRG,agency:IAC,1.8645,1.5651,0.2994
2020-06-25T00:00:00 GPST,R01,agency:GRG,agency:IAC,-3.5314,0.5735,-4.1048
2020-06-25T00:00:00 GPST,E01,agency:GRG,agency:IAC,2.7667,2.3704,0.3963
```

## Outputs

`QcOutput::ClockResidual(QcClockResidual)`, one per clock and per evaluation:

| Field | Description |
|-------|-------------|
| `clock` | `QcClockId::Satellite(sv)` or `QcClockId::Station(name)` |
| `lhs_source`, `lhs_kind` | Product A: agency, kind (`PreciseClock`, `PreciseState`, `Broadcast`) |
| `rhs_source`, `rhs_kind` | Product B |
| `residual_s` | Raw residual A − B (s) |
| `datum_s` | Clock datum difference at this epoch (s), when estimated |
| `aligned_s()` | `residual_s − datum_s` |

## Results

2020-06-25, verified by the test suite (`src/tests/clock`). RMS over the day.

| A − B | Sampling | Constellation | Raw | Aligned |
|-------|----------|---------------|-----|---------|
| GRG (CLK) − IAC (SP3) | 15' | GPS | 1.54 ns | **0.34 ns** |
| | | Galileo | 2.55 ns | **0.23 ns** |
| | | Glonass | 2.79 ns | 2.45 ns |
| GRG (CLK) − GRG (SP3) | 15' | all | < 1 ps | < 1 ps |
| GRG (CLK) − GRG (SP3) | 30" | GPS | 0.13 ns | 0.13 ns |
| GRG (CLK) − ESBC00DNK (NAV) | 15' | GPS | 2.16 ns | 2.16 ns |
| | | Galileo | 0.76 ns | 0.59 ns |

- Two centers: once the datum is removed, GPS and Galileo clocks agree at the 0.1 ns
  level (3 to 10 cm). The Glonass clocks of each center absorb satellite specific biases
  (inter frequency code biases of the FDMA signals): they differ at the ns level, satellite
  per satellite.
- Same center: the Clock RINEX and the SP3 describe the same clocks (same analysis); at 30"
  the residual is the interpolation error of the 15' SP3 clocks (4 cm RMS for GPS, see
  [precise clocks](../clocks.md#results)).
- Broadcast clocks: 2 ns for GPS (0.6 m), 0.6 ns for Galileo.

## Limitations

- No interpolation across data gaps, no extrapolation.
- Ground station clocks are only described by Clock RINEX: station residuals require two
  Clock RINEX.
- The datum is a median: with very few common satellites (< 3 by default), no datum.

[← Residual analysis](README.md)
