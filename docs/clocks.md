Precise clocks
==============

[← Context definition](context.md) · [README](../README.md) · [Meteo observations →](meteo.md)

Clock RINEX files describe the state of clocks, estimated by an analysis center
(IGS, CODE, CNES/CLS..):

- the **onboard clocks of the satellites** (`AS` records),
- the **clocks of ground stations**, receivers of the analysis network (`AR` records).

gnss-qc accepts them as inputs: they are synchronized like every other context product,
streamed out by the `QcPreciseClockProcessor`, and contribute to the
[navigation](navigation/README.md) as satellite clock provider.

Contents:

- [Loading](#loading)
- [Synchronization](#synchronization)
- [Navigation: satellite clocks](#navigation-satellite-clocks)
- [Precise clocks stream](#precise-clocks-stream)
- [Clock residuals](#clock-residuals)
- [Results](#results)
- [Limitations](#limitations)

## Loading

Clock RINEX are loaded like any other file. They are indexed by agency (the analysis center
code of the `ANALYSIS CENTER` header, for example `GRG` or `COM`), see
[indexing](context.md#indexing):

```rust,no_run
use gnss_qc::prelude::*;

let mut products = QcInputProducts::default();

for file in [
    "data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz",
    "data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz",
    "data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz",
    "data/CLK/V3/GRG0MGXFIN_20201770000_01D_30S_CLK.CLK.gz",
] {
    products.load_gzip_file(&QcInputConfig::default(), file).unwrap();
}

// agency:GRG
for (agency, _) in products.clk_rinex_iter() {
    println!("{}", agency);
}
```

Each Clock RINEX becomes a `QcRinexClockSource`, which delivers one
`QcPayload::PreciseClock` per clock and per epoch:

| `QcPreciseClock` | Description |
|------------------|-------------|
| `clock` | `QcClockId::Satellite(SV)` or `QcClockId::Station(name)` |
| `profile` | `AS`, `AR` (analysis results), `CR`, `DR`, `MS` |
| `epoch` | State epoch |
| `offset_s` | Clock offset (s) |
| `offset_dev_s` | Standard deviation of the clock offset (s), if described |
| `drift_s_s` | Clock drift (s/s), if described |

Real-time clock streams (RTCM SSR for example) can be presented to the engine as well:
any source declaring `QcSourceDescriptor::with_precise_clocks()`.

## Synchronization

The engine buffers the analysis results (`AS`, `AR`) in the `QcClockStore`,
per source and per clock, around the epoch being processed (1 hour retention):

| `QcClockStore` | Description |
|----------------|-------------|
| `satellite_offset_s(sv, t)` | Satellite clock offset at `t`, from the first source that can |
| `station_offset_s(name, t)` | Ground station clock offset at `t` |
| `interpolate(clock, t)`, `interpolate_from(source, clock, t)` | Any clock, any source / a given source |
| `samples(source, clock)` | Buffered states, time ordered |
| `clocks_of(source)`, `sources()` | Content of the store |

Clocks are **linearly interpolated** between the two surrounding states: no extrapolation,
no interpolation across a data gap (an interval exceeding 3 times the neighboring sampling
intervals). The store is available to every processor (`QcEpochContext::precise_clocks`):

```rust,no_run
use gnss_qc::prelude::*;

struct MyProcessor;

impl QcProcessor<QcOutput> for MyProcessor {
    fn name(&self) -> &str {
        "my-processor"
    }

    fn process(&mut self, ctx: &QcEpochContext, _: &mut QcOutputQueue<QcOutput>) {
        let g01 = SV::new(Constellation::GPS, 1);

        if let Some((agency, offset_s)) = ctx.precise_clocks.satellite_offset_s(g01, ctx.epoch) {
            println!("{} {} {} {:.3} ns", ctx.epoch, g01, agency, offset_s * 1.0E9);
        }
    }
}
```

## Navigation: satellite clocks

The navigation solver needs the satellite clock offsets. Three products may provide them:

| `QcClockKind` | Product | Sampling | Interpolation |
|---------------|---------|----------|---------------|
| `PreciseClock` | Clock RINEX | 30" (5" for some products) | linear |
| `PreciseState` | SP3 clock states | 5' to 15' | linear |
| `Broadcast` | radio message (NAV) | polynomial, updated every 2 hours (GPS) | none |

The `QcClockPreference` of the [navigation configuration](navigation/README.md#configuration)
selects them:

| `QcClockPreference` | Satellite clock |
|---------------------|-----------------|
| `Auto` (default) | follows the orbit, so both come from consistent products: precise orbit → Clock RINEX when it covers the satellite, SP3 clock states otherwise; broadcast orbit → radio message. A precise orbit is only selected when a precise clock is available |
| `Precise` | precise clocks whatever the orbit: Clock RINEX preferred to the SP3 clock states. Satellites without precise clock are not used |
| `Broadcast` | radio message whatever the orbit. Satellites without broadcast ephemeris are not used |

With both an SP3 and a Clock RINEX, the **Clock RINEX is always preferred** to the SP3 clock
states: denser, it is not affected by the interpolation error of 15' clock states
([results](#results)).

```rust,no_run
use gnss_qc::prelude::*;

// PPP: SP3 orbits + Clock RINEX clocks (default preferences)
let ppp = QcNavigationConfig::default().with_method(Method::PPP);

// broadcast orbits, precise clocks
let cfg = QcNavigationConfig::default()
    .with_orbits(QcOrbitPreference::BroadcastOnly)
    .with_clocks(QcClockPreference::Precise);

// precise orbits, broadcast clocks
let cfg = QcNavigationConfig::default()
    .with_orbits(QcOrbitPreference::PreciseOnly)
    .with_clocks(QcClockPreference::Broadcast);
```

The relativistic correction is applied to every clock (from the orbit in use). The
navigation report counts the clock states served per product (`precise_clocks`,
`precise_state_clocks`, `broadcast_clocks`), and `QcNavigator::clock_kind` tells which
product was used for a satellite. The CGGTTS tracks use the same clocks
(`REFSV`, `REFSYS`).

Orbits and clocks should come from the **same analysis center**: each center has its own
clock datum and orbit realization, a mix of two centers is inconsistent at the decimeter
level. Mixing broadcast orbits with precise clocks (or the opposite) is possible, but
orbit and clock errors no longer compensate each other.

## Precise clocks stream

`QcPreciseClockProcessor` streams the clock states out (`QcOutput::PreciseClock`), once, in
chronological order of each clock, time tagged with the state epoch:

| `QcPreciseClockConfig` | Default | Description |
|------------------------|---------|-------------|
| `satellites` | true | Onboard clocks of the satellites |
| `stations` | true | Clocks of the ground stations |
| `sampling` | every state | Minimal interval between two states of a clock |

```rust,no_run
use gnss_qc::prelude::*;

let mut products = QcInputProducts::default();

products
    .load_gzip_file(
        &QcInputConfig::default(),
        "data/CLK/V3/COD0MGXFIN_20241280000_01D_30S_CLK.CLK.gz",
    )
    .unwrap();

// context only session: satellite clocks, every 5'
let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(QcPreciseClockProcessor::new(
        QcPreciseClockConfig::satellites_only().with_sampling(Duration::from_seconds(300.0)),
    ))
    .build()
    .unwrap();

for item in engine {
    if let QcOutput::PreciseClock(clock) = item.unwrap().payload {
        println!(
            "{} {} {} {:.6} us",
            clock.state.epoch,
            clock.source,
            clock.state.clock,
            clock.state.offset_s * 1.0E6
        );
    }
}
```

```bash
cargo run --release --example precise_clocks > clocks.csv
```

## Clock residuals

Clock products of two analysis centers (two Clock RINEX, a Clock RINEX and an SP3), or
precise and broadcast clocks, are compared by the `QcClockResidualProcessor`: A − B per
satellite and per station, with the clock datum of each center removed.
See [clock residuals](residuals/clocks.md).

## Results

Verified by the test suite (`src/tests/clock`, `src/tests/navigation/scenarios.rs`),
2020-06-25, ESBC00DNK, GRG products.

**Clock RINEX and SP3 of the same analysis center** — at the SP3 epochs (15'), the GRG
Clock RINEX and the GRG SP3 describe the same clock states, within the resolution of the
SP3 format (1 ps): same analysis, same clock datum (> 7000 states compared).

**Interpolation of 15' clock states** — linear interpolation of the clock states every 15'
(SP3 sampling), compared to the 30" Clock RINEX:

| Constellation | RMS | Max |
|---------------|-----|-----|
| GPS | 3.9 cm | 37 cm |
| Galileo | 0.3 cm | 2.6 cm |
| Glonass | 4.3 cm | 36 cm |

The passive hydrogen masers of Galileo are stable enough for 15' states, the GPS and
Glonass clocks are not: this is why Clock RINEX are preferred.

**Navigation** (static, GPS + Galileo):

| Scenario | Orbits | Clocks | 3D RMS |
|----------|--------|--------|--------|
| CPP, 1 h (after 10') | broadcast | broadcast | 2.35 m |
| CPP, 1 h (after 10') | broadcast | Clock RINEX | 1.27 m |
| CPP, 1 h (after 10') | SP3 | SP3 | 1.30 m |
| CPP, 1 h (after 10') | SP3 | Clock RINEX | 1.30 m |
| PPP float, 3 h (after 1h30, ITRF2014) | SP3 | SP3 | 7.1 cm |
| PPP float, 3 h (after 1h30, ITRF2014) | SP3 | Clock RINEX | 8.0 cm |
| PPP float, 3 h (after 1h30, ITRF2014) | SP3 | broadcast | 48 cm |

Precise clocks halve the broadcast navigation error, even with broadcast orbits.
With the same analysis center, Clock RINEX and SP3 clocks give equivalent float PPP
solutions over 3 hours (centimeter level): the interpolation error of the 15' states
(random over satellites) is absorbed by the float solution. Broadcast clocks prevent PPP at
the centimeter level.

## Limitations

- Station clocks (`AR`) are synchronized and streamed, but not used by the navigation:
  the receiver clock is always resolved by the solver.
- No extrapolation: the first epoch of a session may lack a clock when the transmission
  instant precedes the first clock state.
- Calibration (`CR`), discontinuity (`DR`) and monitoring (`MS`) records are delivered by
  the source but not buffered.
- Clock RINEX of distinct analysis centers are not aligned to a common datum.

Next: [Meteo observations →](meteo.md)
