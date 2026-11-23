RTK: differential navigation
============================

[← PPP](ppp.md) · [Navigation](README.md) · [CGGTTS →](cggtts.md)

**Differential navigation** relative to a reference station (*base*) of known coordinates:
double differences (between satellites and between receivers) cancel the satellite clocks,
most of the orbital errors and, for short baselines, the atmospheric delays.

| Method | Signals | Typical use |
|--------|---------|-------------|
| RTK-SPP | single frequency code | low cost receivers, short baselines |
| RTK-CPP | dual frequency code | robust decimeter navigation |
| RTK-PPP | dual frequency code and phase | best geometric accuracy |

- Only the position is resolved (no receiver clock).
- Solutions are expressed in the frame of the base coordinates.
- The baseline (rover-base distance) should remain short: the tests use ~50 km.

## The MISO approach

The base and the rover are two **signal sources** of the same engine. They are synchronized
epoch by epoch: each epoch presents the observations of both receivers. Their identity
includes the receiver serial number, so two stations using the same receiver model remain
two sources (ESBC00DNK and MOJN00DNK both use a SEPT POLARX5).

The base position is taken from its source descriptor (RINEX header: marker position and
antenna eccentricities), unless specified in the configuration
(`QcNavigationTechnique::Rtk { base, position_ecef_m: Some(..) }`).

Epochs without base observations are not solved (counted as `rtk base missing` in the report).

## Inputs

| Input | Role |
|-------|------|
| Rover observations | signals of the receiver to navigate |
| Base observations | signals of the reference station, same epochs |
| NAV | broadcast orbits and clocks |
| SP3 (optional) | precise orbits and clocks |

## Example: RTK-PPP

[examples/rtk_ppp.rs](../../examples/rtk_ppp.rs)

```rust,no_run
use gnss_qc::prelude::*;

let mut products = QcInputProducts::default();

for file in [
    "data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz", // rover
    "data/CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz", // base
    "data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz",
    "data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz",
] {
    products.load_gzip_file(&QcInputConfig::default(), file).unwrap();
}

let rover = QcIndexing::Receiver("SEPT POLARX5 #3047937".to_string());
let base = QcIndexing::Receiver("SEPT POLARX5 #3022663".to_string());

let cfg = QcNavigationConfig::default()
    .with_method(Method::PPP) // RTK-PPP: Method::SPP for RTK-SPP, Method::CPP for RTK-CPP
    .with_rover(rover)
    .with_rtk_base(base)
    .with_user_parameters(UserParameters::new(UserProfile::Static, ClockProfile::Quartz));

let navigation = QcNavigationProcessor::new(MetaAlmanac::latest().unwrap(), cfg).unwrap();

let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(navigation)
    .build()
    .unwrap();

for item in engine {
    if let QcOutput::PVTSolution(solution) = item.unwrap().payload {
        assert_eq!(solution.solution.solution_type, PVTSolutionType::RTK);
        println!("{} {:?}", solution.solution.epoch, solution.solution.pos_m);
    }
}
```

```bash
cargo run --release --features navigation --example rtk_ppp > rtk_ppp.csv
```

## Results

ESBC00DNK (rover), MOJN00DNK (base, ~50 km), 2020-06-25 00:00-01:00, GPS + Galileo,
121/121 epochs solved, errors after 10' against the ETRF2000 header coordinates
(frame of the base coordinates):

| Method | Products | Max horizontal | Max vertical | RMS 3D |
|--------|----------|----------------|--------------|--------|
| RTK-CPP | NAV | 0.37 m | 0.67 m | 0.41 m |
| RTK-PPP | NAV + SP3 | 0.26 m | 0.25 m | 0.31 m |

## Real-time RTK

In real-time, the base observations usually arrive through a network stream (NTRIP / RTCM).
Decode them in a thread and feed the engine with a `QcChannelSource` whose descriptor
declares the base identity, signals and reference position: the engine waits for both
receivers before processing an epoch. See [real-time operation](../miso.md#real-time-operation).
