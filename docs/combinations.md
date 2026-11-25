Signal combinations
===================

The `QcCombinationProcessor` combines the signals of each satellite, for each receiver (signal
source: Observation RINEX, CRINEX, RTCM / NTRIP streams..), epoch after epoch:

| Combination | Measures | Applications |
|-------------|----------|--------------|
| Geometry Free (GF) | Ionospheric delay difference between two frequencies (+ biases, + ambiguities for the phase) | Ionosphere (slant TEC), cycle slips, scintillation, inter-frequency biases |
| Ionosphere Free (IF) | Range without the first order ionospheric delay | What dual frequency navigation (CPP, PPP) actually observes, IF code noise |
| Code multipath (MP) | Code multipath and noise (+ a constant bias per phase arc) | Antenna environment, receiver quality, signal comparison |

- [Signal selection](#signal-selection)
- [Geometry Free](#geometry-free)
- [Ionosphere Free](#ionosphere-free)
- [Code multipath](#code-multipath)
- [Configuration](#configuration)
- [Outputs](#outputs)
- [Example: ESBC00DNK](#example-esbc00dnk)

## Signal selection

For each satellite, the signals are sorted per frequency band (RINEX band number: `1`, `2`,
`5`, `6`, `7`, `8`). On each band, one code and one phase are selected, by order of preference
of their RINEX attribute: `C`, `W`, `P`, `Y`, `X`, `Q`, `I`, `L`, `S`, `B`, `D`, `M`, `Z`, `A`, `N`
(civilian signals first).

The **reference signal (signal 1)** is the lowest band number: L1 / E1 / B1C, or B1I (`2I`)
for BeiDou receivers that do not track B1C. **Every other band j** is combined with it, not only
L2 or L5: GPS L1/L2 and L1/L5, Galileo E1/E5a, E1/E5b, E1/E5 (AltBOC), E1/E6, BeiDou B1I/B3I,
B1I/B2I..

All values are expressed in meters: phases (cycles) are converted with the wavelength of their
carrier. Glonass (FDMA: each satellite has its own frequencies) is not supported.

## Geometry Free

The geometry (ranges, clocks, troposphere) cancels out, the ionospheric delay difference remains:

| | Formula | Equals |
|-|---------|--------|
| Phase | `GF = Lj − L1` | `I1 − Ij` + inter-frequency biases + ambiguities |
| Code | `GF = C1 − Cj` | `I1 − Ij` + inter-frequency biases |

The phase convention (Lj − L1) and the code convention (C1 − Cj) are opposite, because the
ionosphere delays the code and advances the phase: both GF combinations measure the same
quantity, `I1 − Ij`, which is negative and proportional to the slant TEC
(`I = 40.3 TEC / f²`). They can be plotted together: the phase GF is much less noisy (mm), but
offset by the ambiguities; the code GF is noisy (multipath) but absolute. Smoothing the code
GF with the phase GF ("leveling") gives the slant TEC.

A jump of the phase GF reveals a cycle slip.

## Ionosphere Free

The first order ionospheric delay cancels out, for the code and for the phase:

```text
IF = (f1² X1 − fj² Xj) / (f1² − fj²)
```

X being the code or the phase (m). This is the combination that dual frequency navigation
(CPP, PPP) processes. The price is the noise: for L1/L2, the IF code is about 3 times noisier
than C1.

## Code multipath

The multipath of code `Ci` (band i) is the code minus a combination of two phases (`Li`, same
band, and `Lj`, another band) that removes the geometry, the clocks, the troposphere and the
first order ionosphere:

```text
MPi = Ci − Li − 2 (Li − Lj) / (αij − 1),      αij = (fi / fj)²
```

For L1/L2 (α = (f1/f2)²), this is the classical pair (TEQC):

```text
MP1 = C1 − (1 + 2 / (α − 1)) L1 + (2 / (α − 1)) L2
MP2 = C2 − (2α / (α − 1)) L1 + (2α / (α − 1) − 1) L2
```

What remains: the code multipath and noise (the phase multipath being negligible), plus a
**constant bias** (phase ambiguities, hardware delays) for each **continuous phase arc**.
Every code of band i is analyzed (`C1C` and `C1W`, `C2W` and `C2L`..), with the phase of the
same band, and the phase of the reference band (or of the next band, for the reference band
itself): `MP C2W (L1C)`, `MP C1C (L2W)`, `MP C5Q (L1C)`..

An arc ends when:

- one of the two phases reports a loss of lock (LLI),
- no data for more than `max_gap` (10 minutes by default): satellite set, data gap,
- the value jumps by more than `jump_threshold_m` (5 m by default) with respect to the mean of
  the arc (undetected cycle slip).

The statistics of each arc are emitted when it ends (`QcMultipathArc`): number of samples,
mean (the bias) and RMS around the mean (the multipath). Each multipath value comes with its arc
number and its value minus the mean of its arc so far (`debiased_m`), to monitor
the multipath in real-time. For an exact debiasing, subtract the mean of the arc
(`QcMultipathArc::mean_m`) once it is known.

## Configuration

```rust
use gnss_qc::prelude::*;

// default: GF, IF (code and phase) and MP, every receiver, every constellation
let all = QcCombinationProcessor::new(QcCombinationConfig::default());

// Galileo phase GF only (ionosphere monitoring), one receiver
let gf = QcCombinationProcessor::new(
    QcCombinationConfig {
        code: false,
        ..Default::default()
    }
    .with_combinations(&[QcCombinationKind::GeometryFree])
    .with_constellations(&[Constellation::Galileo])
    .with_receiver(QcIndexing::Receiver("SEPT POLARX5 #3047937".to_string())),
);
```

| Field | Default | Description |
|-------|---------|-------------|
| `geometry_free` | ✓ | GF combinations |
| `ionosphere_free` | ✓ | IF combinations |
| `multipath` | ✓ | Code multipath |
| `code` | ✓ | Combine pseudo ranges (GF, IF) |
| `phase` | ✓ | Combine phase ranges (GF, IF) |
| `receivers` | all | Receivers to analyze |
| `constellations` | all | Constellations to analyze (Glonass excluded) |
| `max_gap` | 10 min | Multipath: a longer gap ends the arc |
| `jump_threshold_m` | 5 m | Multipath: a larger jump ends the arc |

## Outputs

`QcOutput::Combination(QcCombination)`, for each epoch, satellite and combination:

| Field | GF | IF | MP |
|-------|----|----|----|
| `kind` | `GeometryFree` | `IonosphereFree` | `Multipath` |
| `lhs` | `Lj` (phase) / `C1` (code) | signal 1 | analyzed code `Ci` |
| `rhs` | `L1` (phase) / `Cj` (code) | signal j | phase of the other band `Lj` |
| `value_m` | `lhs − rhs` | IF combination | raw multipath (with the arc bias) |
| `arc` | | | arc number |
| `debiased_m` | | | value minus the mean of the arc so far |

`QcCombination::name()` gives a short name: `GF L2W-L1C`, `GF C1C-C2W`, `IF L1C-L5Q`,
`MP C2W (L1C)`.

`QcOutput::MultipathArc(QcMultipathArc)`, when an arc ends: receiver, satellite, code, phase,
arc number, start, end, samples, `mean_m`, `rms_m`.

```rust,no_run
use gnss_qc::prelude::*;

let engine = QcEngine::<QcOutput>::builder()
    // .with_products(products) or .with_source(stream)
    .with_processor(QcCombinationProcessor::new(QcCombinationConfig::default()))
    .build()
    .unwrap();

for item in engine {
    let item = item.unwrap();
    match item.payload {
        QcOutput::Combination(c) => {
            println!("{} {} {} {} {:.3}", item.epoch, c.receiver, c.sv, c.name(), c.value_m);
        },
        QcOutput::MultipathArc(arc) => {
            println!(
                "{} {} {}: arc #{} {} - {}, RMS {:.3} m",
                arc.receiver, arc.sv, arc.code, arc.arc, arc.start, arc.end, arc.rms_m
            );
        },
        _ => {},
    }
}
```

## Example: ESBC00DNK

ESBC00DNK (Septentrio PolaRx5, 2020-06-25, 24h, 30 s), GPS, Galileo, BeiDou and QZSS
(see `src/tests/combinations.rs`):

- every pair of frequencies is combined: GPS L1/L2 and L1/L5, Galileo E1/E5a, E1/E5b, E1/E5,
  E1/E6, BeiDou B1I/B3I and B1I/B2I, QZSS L1/L2 and L1/L5;
- GF code − GF phase (GPS L1/L2) is constant within a pass, with a median dispersion of 0.29 m
  (code noise): both measure the same ionospheric delay;
- code multipath RMS per arc (median, GPS): MP C1C (L2W) 0.29 m, MP C2W (L1C) 0.28 m,
  MP C5Q (L1C) 0.27 m.
