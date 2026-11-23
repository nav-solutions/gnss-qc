CPP: Code Precise Positioning
=============================

[← SPP](spp.md) · [Navigation](README.md) · [PPP →](ppp.md)

**Dual frequency code navigation**: the iono free combination of two pseudo ranges
(GPS L1 / L2, Galileo E1 / E5a, BeiDou B1I / B3I) cancels the first order ionospheric delay.
Absolute navigation (position and receiver clock). Phase observations are disregarded.

- Dual frequency receivers, timing applications, robust navigation.
- Meter to decimeter level, depending on the orbital products.
- No convergence period: every epoch is solved independently from the phase history.

## Inputs

| Input | Role |
|-------|------|
| Observations | dual frequency pseudo ranges |
| NAV | broadcast orbits and clocks (fallback when precise products are missing) |
| SP3 (recommended) | precise orbits and clocks |

## Example

[examples/cpp.rs](../../examples/cpp.rs)

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

let cfg = QcNavigationConfig::default()
    .with_method(Method::CPP)
    .with_orbits(QcOrbitPreference::Auto)
    .with_user_parameters(UserParameters::new(UserProfile::Static, ClockProfile::Quartz));

let navigation = QcNavigationProcessor::new(MetaAlmanac::latest().unwrap(), cfg).unwrap();

let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(navigation)
    .build()
    .unwrap();

for item in engine {
    match item.unwrap().payload {
        QcOutput::PVTSolution(solution) => {
            println!("{} {:?} {:e}", solution.solution.epoch, solution.solution.pos_m, solution.solution.clock_offset_s)
        },
        QcOutput::NavigationReport(report) => {
            println!("precise orbits: {}, broadcast orbits: {}", report.precise_orbits, report.broadcast_orbits)
        },
        _ => {},
    }
}
```

```bash
cargo run --release --features navigation --example cpp > cpp.csv
```

## Results

ESBC00DNK, 2020-06-25 00:00-01:00, GPS + Galileo, 121/121 epochs solved, errors after 10'
against the ETRF2000 header coordinates (which are 0.75 m away from the ITRF2014 frame of
the solutions):

| Products | Max horizontal | Max vertical | RMS 3D |
|----------|----------------|--------------|--------|
| NAV | 1.8 m | 2.4 m | 2.4 m |
| NAV + SP3 | 1.2 m | 1.2 m | 1.3 m |

With NAV + SP3, more than 99% of the orbital states come from the precise products:
broadcast ephemerides are only used at the first epoch (the transmission instant precedes
the first SP3 sample).

## Precise products coverage

When the precise products end before the observations, `QcOrbitPreference::Auto` switches to
the broadcast ephemerides, satellite per satellite, without interrupting the solutions stream.
`QcOrbitPreference::PreciseOnly` stops navigating instead. Both behaviors are verified by the
`absolute_cpp_precise_coverage_loss` test (SP3 truncated at 03:00, observations until 03:30).
