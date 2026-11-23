SPP: Single Point Positioning
=============================

[← Navigation](README.md) · [CPP →](cpp.md)

**Single frequency code navigation**: one pseudo range per satellite (GPS L1 C/A,
Galileo E1, BeiDou B1I..), absolute navigation (position and receiver clock).

- Low cost, single frequency receivers, degraded setups.
- Meter level accuracy: the ionospheric delay is not cancelled (and not modeled yet).
- Single frequency signals require the broadcast group delays (TGD / BGD): they are applied
  automatically, whatever the orbital product (broadcast and precise clocks are referenced
  to the iono free combination). Navigation files are therefore required, even with SP3.

## Inputs

| Input | Role |
|-------|------|
| Observations | L1 / E1 / B1 pseudo ranges (other signals are disregarded) |
| NAV | broadcast orbits and clocks, group delays |
| SP3 (optional) | precise orbits and clocks |

## Example

[examples/spp.rs](../../examples/spp.rs)

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
    .with_method(Method::SPP)
    .with_constellations(&[Constellation::GPS, Constellation::Galileo])
    .with_user_parameters(UserParameters::new(UserProfile::Static, ClockProfile::Quartz));

let navigation = QcNavigationProcessor::new(MetaAlmanac::latest().unwrap(), cfg).unwrap();

let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(navigation)
    .build()
    .unwrap();

for item in engine {
    if let QcOutput::PVTSolution(solution) = item.unwrap().payload {
        let (lat, long, alt) = solution.solution.lat_long_alt_deg_deg_m;
        println!("{} {:.7} {:.7} {:.1}", solution.solution.epoch, lat, long, alt);
    }
}
```

```bash
cargo run --release --features navigation --example spp > spp.csv
```

## Results

ESBC00DNK, 2020-06-25 00:00-01:00, GPS + Galileo, NAV + SP3: 121/121 epochs solved,
5.5 m max horizontal error, 3.6 m max vertical error, 5.4 m RMS (after 10').

For better accuracy with a single frequency receiver, prefer [RTK-SPP](rtk.md):
the ionospheric delay cancels between two close receivers.
