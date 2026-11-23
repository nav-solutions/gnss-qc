Navigation
==========

[← Processors](../processors.md) · [README](../../README.md)

The `navigation` feature resolves **P.V.T solutions** (Position, Velocity, Time) with the
[GNSS-RTK](https://github.com/nav-solutions/gnss-rtk) solver. Navigation is a processor
of the [MISO engine](../miso.md): the solutions are streamed like any other result.

```text
 OBS (rover) ─────┐                                   ┌─ candidates (rover, base)
 OBS (RTK base) ──┤                                   │
 NAV ─────────────┼──► QcEngine ──► QcNavigationProcessor ──► PVT solutions stream
 SP3 ─────────────┘   (synchronized       │           └─ orbits + clocks: SP3 > NAV,
                       context)           │              per satellite, per epoch
                                          └─ GNSS-RTK Solver (ANISE almanac)
```

| Method | Signals | Guide | Example |
|--------|---------|-------|---------|
| SPP | single frequency code | [SPP](spp.md) | [spp.rs](../../examples/spp.rs) |
| CPP | dual frequency code (iono free) | [CPP](cpp.md) | [cpp.rs](../../examples/cpp.rs) |
| PPP | dual frequency code and phase (iono free) | [PPP](ppp.md) | [ppp.rs](../../examples/ppp.rs) |
| RTK-SPP, RTK-CPP, RTK-PPP | same, differential (reference station) | [RTK](rtk.md) | [rtk_ppp.rs](../../examples/rtk_ppp.rs) |
| CGGTTS | SPP, CPP or PPP, post-fitted (`cggtts` feature) | [CGGTTS](cggtts.md) | [cggtts.rs](../../examples/cggtts.rs) |

## Deployment

Every navigation session follows the same four steps:

```rust,no_run
use gnss_qc::prelude::*;

// 1. Context: observations, ephemerides, precise products
let mut products = QcInputProducts::default();
let cfg = QcInputConfig::default();

for file in [
    "data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz",
    "data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz",
    "data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz",
] {
    products.load_gzip_file(&cfg, file).unwrap();
}

// 2. Navigation processor
let almanac = MetaAlmanac::latest().unwrap(); // ANISE data (see below)

let cfg = QcNavigationConfig::default()
    .with_method(Method::CPP)
    .with_constellations(&[Constellation::GPS, Constellation::Galileo])
    .with_user_parameters(UserParameters::new(UserProfile::Static, ClockProfile::Quartz));

let navigation = QcNavigationProcessor::new(almanac, cfg).unwrap();

// 3. Engine
let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(navigation)
    .build()
    .unwrap();

// 4. Solutions stream
for item in engine {
    match item.unwrap().payload {
        QcOutput::PVTSolution(solution) => {
            let pvt = &solution.solution;
            println!("{} {:?} clock={:e}s gdop={}", pvt.epoch, pvt.pos_m, pvt.clock_offset_s, pvt.gdop);
        },
        QcOutput::NavigationReport(report) => println!("{:#?}", report),
        _ => {},
    }
}
```

## Inputs

| Input | Required | Provides |
|-------|----------|----------|
| Observations (OBS / CRINEX) | yes | pseudo ranges, phase ranges, SNR (rover, and base in RTK) |
| Navigation (NAV) | yes, unless precise products cover every satellite | Keplerian orbits and clocks, group delays (TGD / BGD) |
| Precise products (SP3) | recommended, required for PPP | precise orbits and clocks |
| ANISE almanac | yes | Earth frame, Sun position (eclipses) |

Supported constellations: GPS, Galileo, BeiDou, QZSS (Glonass and SBAS are not supported
by the solver yet).

### ANISE almanac

The solver requires an ANISE `Almanac`. Use the re-exported `MetaAlmanac` (same ANISE version
as the solver):

| Method | Description |
|--------|-------------|
| `MetaAlmanac::latest()` | Downloads the latest dataset (de440s.bsp, pck11.pca..) on first use, cached afterwards |
| `Almanac::new("de440s.bsp")?.load("pck11.pca")?` | Local files (offline deployments) |
| `embed_ephem` feature | Dataset embedded at build time |

## Orbital products

When precise products (SP3) and broadcast ephemerides (NAV) are both available,
`QcOrbitPreference` defines how they are combined:

| Preference | Behavior |
|------------|----------|
| `Auto` (default) | Precise products whenever they cover the satellite, broadcast ephemerides otherwise. **Decided per satellite and per epoch** |
| `PreciseOnly` | Precise products only: satellites without precise products are not used |
| `BroadcastOnly` | Broadcast ephemerides only |

Precise products *cover* a satellite when both its position and its clock can be interpolated
at the sampling instant and at the transmission instant (no extrapolation, no data gap).
Consequently, in `Auto` mode, broadcast ephemerides take over transparently:

- at the very beginning of the products (the transmission instant precedes the first sample);
- when a satellite is missing from the precise products (BeiDou in a GPS + Galileo product for example);
- when the precise products end before the observations (the solutions stream continues without interruption).

The orbit and the clock of one satellite always come from the same product: mixing a precise
orbit with a broadcast clock would be inconsistent at the meter level.

| Correction | Broadcast | Precise |
|------------|-----------|---------|
| Orbit | Keplerian solving (GPS, Galileo, QZSS, BeiDou MEO / IGSO / GEO) | Lagrange interpolation (order 10) |
| Clock | polynomial (a0, a1, a2) | linear interpolation between samples |
| Relativistic clock correction | F.e.√A.sin(E) | -2 r·v / c² |
| Group delay (single frequency only) | broadcast TGD / BGD | broadcast TGD / BGD (precise clocks are iono free referenced) |

Single frequency navigation (SPP) only proposes the primary signal of each constellation
to the solver (GPS / QZSS L1, Galileo E1, BeiDou B1I): the signal the broadcast group
delays are defined for.

The `QcNavigationReport`, emitted at the end of the session, counts the orbital states
served from each product (`precise_orbits`, `broadcast_orbits`).

## Configuration

`QcNavigationConfig`:

| Field | Builder | Default | Description |
|-------|---------|---------|-------------|
| `technique` | `with_rtk_base(base)` | `Absolute` | `Absolute` or `Rtk { base, position_ecef_m }` |
| `rover` | `with_rover(indexing)` | first signal source (that is not the base) | Receiver to navigate |
| `constellations` | `with_constellations(..)` | GPS, Galileo | Constellations to navigate with |
| `orbits` | `with_orbits(..)` | `Auto` | Orbital products preference |
| `solver` | `with_method(method)` | GNSS-RTK defaults, CPP | Solver configuration (`QcSolverConfig`): method, models, filters |
| `user` | `with_user_parameters(..)` | Pedestrian, Quartz | User profile (dynamics, clock) |
| `apriori_ecef_m` | - | none (survey) | A priori position (ECEF m) |

The user profile helps the navigation filter: `UserProfile::Static` for geodetic stations,
`Pedestrian`, `Car`, `Airplane`.. for moving receivers.

The complete solver configuration is accessible through `cfg.solver` (see the GNSS-RTK
documentation). For example, the post-fit smoother, which may hold the published solution
for a few epochs, can be disabled to observe the navigation filter itself:

```rust,no_run
use gnss_qc::prelude::*;

let mut cfg = QcNavigationConfig::default().with_method(Method::PPP);
cfg.solver.solver.postfit_denoising = 0.0;
```

## Models

| Perturbation | Model |
|--------------|-------|
| Troposphere | Niell |
| Ionosphere | none yet: cancelled by the iono free combination (CPP, PPP), ignored in SPP |
| Earth rotation, eclipses | GNSS-RTK (ANISE) |
| Antenna phase centers, tides | not modeled yet |

## Outputs

| Output | Description |
|--------|-------------|
| `QcOutput::PVTSolution(QcPvtSolution)` | `receiver` + GNSS-RTK `PVTSolution`: position and velocity (ECEF), geodetic coordinates, clock offset (absolute navigation), DOPs, contributing satellites. `solution_type` is `PPP` for any absolute solution, `RTK` for differential solutions |
| `QcOutput::NavigationReport(QcNavigationReport)` | End of session: epochs, solutions, failures (per reason), orbital states per product |

Solutions are only streamed when the solver succeeds; failures are counted in the report.

## Reference frames

Absolute solutions obtained with IGS precise products are expressed in the frame of the
products (ITRF2014 / IGS14 for the GRG products of the examples). Station coordinates
(RINEX headers, national networks) may be expressed in another frame: ETRF2000 in Europe,
0.75 m away from ITRF2014 in Denmark in 2020. `gnss_qc::geodesy::etrf2000_to_itrf2014`
converts them. Differential (RTK) solutions are expressed in the frame of the base coordinates.

## Performances

ESBC00DNK (rover) and MOJN00DNK (base, ~50 km), 2020-06-25, 30 s, GPS + Galileo,
GRG precise products. Errors after convergence, verified by the test suite
(`src/tests/navigation/scenarios.rs`):

| Method | Products | Period | Reference | Max horizontal | Max vertical | RMS 3D |
|--------|----------|--------|-----------|----------------|--------------|--------|
| SPP | NAV + SP3 | 1h (after 10') | ETRF2000 | 1.1 m | 1.4 m | 1.7 m |
| CPP | NAV | 1h (after 10') | ETRF2000 | 1.8 m | 2.4 m | 2.4 m |
| CPP | NAV + SP3 | 1h (after 10') | ETRF2000 | 1.2 m | 1.2 m | 1.3 m |
| PPP | NAV + SP3 | 3h (after 1h30) | ITRF2014 | 0.09 m | 0.05 m | 0.07 m |
| RTK-CPP | NAV | 1h (after 10') | ETRF2000 | 0.37 m | 0.67 m | 0.41 m |
| RTK-PPP | NAV + SP3 | 1h (after 10') | ETRF2000 | 0.26 m | 0.25 m | 0.31 m |

The CPP errors against ETRF2000 include the 0.75 m frame offset. Processing time (release
build, decompression and parsing included): ~1.5 s per hour of data.

## Limitations

- Glonass and SBAS are not supported.
- No ionosphere model: SPP accuracy depends on the ionospheric activity (meter level here,
  night time at solar minimum, several meters otherwise).
- Precise clocks are interpolated from the SP3 (15'): Clock RINEX (30 s) are not supported yet,
  they would improve PPP.
- PPP ambiguities are float (no integer ambiguity resolution yet).
- Antenna phase centers (ANTEX) are not applied yet.
