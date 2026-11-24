Residual analysis
=================

[← Processors](../processors.md) · [README](../../README.md) · [Observation residuals →](observations.md) · [Orbit residuals →](orbits.md) · [Clock residuals →](clocks.md)

Residual analyses differentiate two products that describe **the same physical quantity**:
A − B. What remains is what differs between both producers: receivers, analysis centers,
broadcast messages.

They only rely on standard files (RINEX, CRINEX, SP3) and are available by default, without
any feature (`sp3` is only needed to load precise products).

| Analysis | Processor | A − B | Reveals |
|----------|-----------|-------|---------|
| [Observation residuals](observations.md) | `QcObservationResidualProcessor` | receiver A − receiver B, same satellite, same observable | relative clock behavior, receiver level artifacts, zero / short baseline quality |
| [Orbit residuals](orbits.md) | `QcOrbitResidualProcessor` | precise − precise (two agencies) | consistency between analysis centers (cm level) |
| | | broadcast − precise | broadcast ephemerides quality (m level) |
| | | broadcast − broadcast (two sources) | decoding / encoding consistency between two receivers or NAV files |
| [Clock residuals](clocks.md) | `QcClockResidualProcessor` | Clock RINEX − Clock RINEX (two agencies), Clock RINEX − SP3, precise − broadcast | consistency of the clock solutions (0.1 ns level), once the clock datum of each center is removed |

## In the MISO engine

These processors are regular `QcProcessor`s: they run on the synchronized epoch context, along
with any other processor, from files or real-time streams.

- **Observation residuals** run on the signal sources: one residual per observation of
  receiver A that has a counterpart in receiver B (same satellite, same observable), at the
  closest epoch of B within a tolerance.
- **Orbit residuals** run on the context (ephemerides, precise products). Without receiver,
  the session is *context only*: the engine iterates over the epochs of the context
  products themselves (precise products samples, broadcast times of clock), so that SP3 and
  NAV files can be compared without any observation file.

```rust,no_run
use gnss_qc::prelude::*;

let mut products = QcInputProducts::default();

for file in [
    "data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz",
    "data/SP3/D/Sta21114.sp3.gz",
] {
    products.load_gzip_file(&QcInputConfig::default(), file).unwrap();
}

// context only session: no receiver
let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(QcOrbitResidualProcessor::new(QcOrbitResidualConfig::precise(
        QcIndexing::Agency("GRGS".to_string()),
        QcIndexing::Agency("IAC".to_string()),
    )))
    .build()
    .unwrap();

for item in engine {
    if let QcOutput::OrbitResidual(r) = item.unwrap().payload {
        println!("{} {:.3} m", r.sv, r.norm_m());
    }
}
```

## Identifying A and B

A and B are designated with the indexing of the [context](../context.md):

| Product | Indexing | Example |
|---------|----------|---------|
| Observations | `QcIndexing::Receiver` ("MODEL #SN" of the RINEX header) | `Receiver("SEPT POLARX5TR #3088149")` |
| Precise products | `QcIndexing::Agency` (SP3 header) | `Agency("GRGS")`, `Agency("IAC")` |
| Broadcast ephemerides | file stem, or a custom alias | `Custom("ESBC00DNK_R_20201770000_01D_MN")` |

Custom aliases (`QcInputConfig`) give explicit names to any product, see
[context definition](../context.md).

## Examples

| Example | Content |
|---------|---------|
| [observation_residuals](../../examples/observation_residuals.rs) | PolaRx5 − GNSS-SDR (common antenna), pseudo range and phase |
| [orbit_residuals](../../examples/orbit_residuals.rs) | GRG − IAC, NAV (ESBC00DNK) − GRG |

```bash
cargo run --release --example observation_residuals > observation_residuals.csv
cargo run --release --example orbit_residuals > orbit_residuals.csv
```

Next: [Observation residuals →](observations.md)
