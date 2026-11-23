PPP: Precise Point Positioning
==============================

[← CPP](cpp.md) · [Navigation](README.md) · [RTK →](rtk.md)

**Dual frequency code and phase navigation**: the iono free combinations of code and phase
are processed by a single navigation filter, estimating the position, the receiver clock,
the zenith wet delay and one float ambiguity per continuous phase arc.

- Geodetic surveying, stations calibration, timing.
- Centimeter level once converged (float ambiguities).
- Requires precise products (SP3): broadcast orbits and clocks are meter level.
- Convergence: the solution starts at the CPP accuracy and converges as the geometry changes
  (tens of minutes). Cycle slips and data gaps restart the affected phase arcs automatically.

## Inputs

| Input | Role |
|-------|------|
| Observations | dual frequency pseudo ranges and phase ranges |
| SP3 | precise orbits and clocks |
| NAV | fallback for satellites not covered by the precise products |

## Example

[examples/ppp.rs](../../examples/ppp.rs)

```rust,no_run
use gnss_qc::{geodesy::etrf2000_to_itrf2014, prelude::*};

let mut products = QcInputProducts::default();

for file in [
    "data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz",
    "data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz",
    "data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz",
] {
    products.load_gzip_file(&QcInputConfig::default(), file).unwrap();
}

let cfg = QcNavigationConfig::default()
    .with_method(Method::PPP)
    .with_constellations(&[Constellation::GPS, Constellation::Galileo])
    .with_user_parameters(UserParameters::new(UserProfile::Static, ClockProfile::Quartz));

let navigation = QcNavigationProcessor::new(MetaAlmanac::latest().unwrap(), cfg).unwrap();

let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(navigation)
    .build()
    .unwrap();

// station coordinates (RINEX header) are ETRF2000, the solutions ITRF2014 (GRG products)
let reference = etrf2000_to_itrf2014((3582105.291, 532589.731, 5232754.805), 2020.48);

for item in engine {
    if let QcOutput::PVTSolution(solution) = item.unwrap().payload {
        let (x, y, z) = solution.solution.pos_m;
        let error = ((x - reference.0).powi(2) + (y - reference.1).powi(2) + (z - reference.2).powi(2)).sqrt();
        println!("{} {:.3} m", solution.solution.epoch, error);
    }
}
```

```bash
cargo run --release --features navigation --example ppp > ppp.csv
```

## Results

ESBC00DNK, 2020-06-25 00:00-03:00, GPS + Galileo, NAV + SP3: 361/361 epochs solved.
After 1h30 of convergence, against the ITRF2014 coordinates of the station:
0.09 m max horizontal error, 0.05 m max vertical error, 0.07 m RMS.

Against the ETRF2000 coordinates of the RINEX header, the horizontal error would be ~0.7 m:
this is the frame offset, not a navigation error. See [reference frames](README.md#reference-frames).

## Tips

- Use `UserProfile::Static` for static receivers: it helps the filter converge.
- The post-fit smoother (`cfg.solver.solver.postfit_denoising`) smooths the published
  solution; set it to `0.0` to observe the navigation filter itself.
- Precise clocks are interpolated from the SP3 samples (15'). Clock RINEX (30 s) are not
  supported yet.
