Observation residuals
=====================

[← Residual analysis](README.md) · [README](../../README.md) · [Orbit residuals →](orbits.md)

`QcObservationResidualProcessor` differentiates the observations of two receivers:
for each epoch of receiver A, each observation is differentiated with the observation of
receiver B for **the same satellite and the same observable** (A − B), at the closest epoch
of B within a tolerance. Observations without counterpart are discarded.

It is the streamed counterpart of `Rinex::observations_substract` of the RINEX library, and
produces the same values (verified by the test suite), but it applies to real-time streams
as well as files, and to receivers sampling at different instants.

## Principle

```text
 OBS receiver A ──┐                                   ┌─ same SV, same observable
                  ├──► QcEngine ──► QcObservationResidualProcessor ─┤
 OBS receiver B ──┘                                   └─ closest epoch of B, |tA − tB| ≤ tolerance
                                                           └──► A − B (per SV, per observable)
```

Receivers that share an antenna (zero baseline) or are very close (short baseline) see the
same geometry and the same propagation delays: the single difference cancels them, and
leaves what is specific to each receiver:

- **pseudo range** residuals (m): the clock offset between both receivers (common to all
  satellites), plus code noise, multipath (short baseline) and inter-receiver code biases;
- **phase** residuals (cycles): the clock offset as well, plus an ambiguity per satellite
  (constant while tracking is continuous), cycle slips, and any receiver level phase
  artifact;
- Doppler and signal strength residuals, on request.

Following the pseudo range residual over time gives the **relative behavior of the local
clocks** (offset, drift); a disagreement between the phase and code trends reveals a
tracking or phase generation problem in one of the receivers.

## Configuration

`QcObservationResidualConfig`:

| Field | Default | Description |
|-------|---------|-------------|
| `lhs` | first receiver encountered | Receiver A |
| `rhs` | first other receiver encountered | Receiver B |
| `pseudo_range` | true | Differentiate pseudo ranges (m) |
| `phase_range` | true | Differentiate phase ranges (cycles) |
| `doppler` | false | Differentiate Doppler shifts (Hz) |
| `ssi` | false | Differentiate signal strengths |
| `tolerance` | half the sampling interval of A | Maximal time difference between the epochs of A and B |

Helpers: `QcObservationResidualConfig::new(a, b)` designates both receivers,
`.with_all_observables()` enables every observable.

With the default tolerance (the semantic of `observations_substract`), the very first epoch
of A requires an exact match since its sampling interval is not known yet.
The epoch of B that was used is reported in each residual (`rhs_epoch`).

## Example

[examples/observation_residuals.rs](../../examples/observation_residuals.rs)

```rust,no_run
use gnss_qc::prelude::*;

let mut products = QcInputProducts::default();

for file in ["data/OBS/V3/OB713520.23O.gz", "data/OBS/V3/gps.23O.gz"] {
    products.load_gzip_file(&QcInputConfig::default(), file).unwrap();
}

// A - B: PolaRx5 - GNSS-SDR, pseudo range and phase
let residuals = QcObservationResidualProcessor::new(QcObservationResidualConfig::new(
    QcIndexing::Receiver("SEPT POLARX5TR #3088149".to_string()),
    QcIndexing::Receiver("Software Receiver #GNSS-SDR".to_string()),
));

let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(residuals)
    .build()
    .unwrap();

for item in engine {
    let item = item.unwrap();
    if let QcOutput::ObservationResidual(r) = item.payload {
        println!("{} {} {} {:.3}", item.epoch, r.sv, r.observable, r.residual);
    }
}
```

```bash
cargo run --release --example observation_residuals > residuals.csv
```

```text
epoch,rhs_epoch,sv,observable,residual
2023-12-18T17:29:00 GPST,2023-12-18T17:29:00 GPST,G25,L1C,-14861261.322
2023-12-18T17:29:00 GPST,2023-12-18T17:29:00 GPST,G25,C1C,-2828022.124
2023-12-18T17:29:00 GPST,2023-12-18T17:29:00 GPST,G26,L1C,-14861393.788
```

## Outputs

`QcOutput::ObservationResidual(QcObservationResidual)`, one per differentiated observation,
at the epoch of A, sorted by satellite and observable:

| Field | Description |
|-------|-------------|
| `lhs`, `rhs` | Receivers A and B |
| `sv` | Satellite |
| `observable` | Observable (RINEX code, e.g. `C1C`, `L1C`) |
| `residual` | A − B, in the unit of the observable (m, cycles, Hz, dB-Hz) |
| `rhs_epoch` | Epoch of B used for the difference |

## Results

Verified by the test suite (`src/tests/residuals`).

**Two geodetic receivers** — ESBC00DNK − MOJN00DNK (2020-06-25, every observable, 30'):
the streamed residuals are identical to `Rinex::observations_substract` (> 30 000 residuals).

**Geodetic receiver − software receiver, common antenna** — Septentrio PolaRx5 −
GNSS-SDR, 2023-12-18 17:29-19:27, common observables C1C and L1C (~1900 residuals):

- the pseudo range residuals are common to all satellites: −2 828 020 m, i.e. a clock
  offset of **−9.433 ms** between both receivers, with no measurable drift over 2 hours
  (< 0.01 m/s: the receivers are syntonized), the spread between satellites remains within
  a few tens of meters (code noise of the software receiver);
- the phase residuals (× λ L1) drift by **−0.48 m/s**, identically on most satellites,
  while the code does not: the phase of one receiver diverges from its code. A drift
  common to all satellites and absent from the code is a receiver level artifact (carrier
  phase generation / tracking loop of the software receiver), not a clock effect — exactly
  the kind of behavior this analysis is meant to reveal.

## Limitations

- Pairs of receivers only (A − B); run several processors for several pairs.
- No interpolation: residuals are formed at the closest epoch of B within the tolerance;
  with distant epochs, the satellite motion is not compensated (pseudo ranges vary up to
  ~800 m/s).
- Raw differences: no ambiguity resolution, no cycle slip detection or correction.
- The observables must match exactly (`C1C` with `C1C`): receivers tracking different codes
  have no common observation.

Next: [Orbit residuals →](orbits.md)
