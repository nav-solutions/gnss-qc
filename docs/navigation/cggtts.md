CGGTTS: common view time transfer
=================================

[← RTK](rtk.md) · [Navigation](README.md)

The `cggtts` feature resolves **CGGTTS solutions**: the standardized format of the
GNSS common view time transfer (BIPM, CCTF), used by the timing laboratories to compare
their clocks. It relies on the [CGGTTS library](https://github.com/nav-solutions/cggtts).

```toml
gnss-qc = { version = "..", features = ["cggtts"] }
```

## Principle

CGGTTS is a post-fit of the absolute navigation solutions:

```text
 OBS + NAV + SP3 ──► QcEngine ──► QcCggttsProcessor
                                   ├─ QcNavigator: SPP / CPP / PPP, every epoch
                                   │   └─ P.V.T solution: clock offset (REFSYS),
                                   │      per satellite: clock correction, troposphere,
                                   │      ionosphere, elevation, azimuth
                                   ├─ SkyTracker: one SVTracker per satellite, fed during
                                   │   the data collection of each tracking period
                                   └─ end of period: linear fit per satellite ──► CGGTTS track
```

1. The receiver is navigated at every epoch, in absolute mode (**SPP, CPP or PPP**):
   the navigation filter runs continuously, whatever the schedule.
2. During the **data collection** of each tracking period, every satellite that contributed
   to the solution feeds its tracker with:

   | CGGTTS field | Content |
   |--------------|---------|
   | `REFSYS` | receiver clock offset to the system time, resolved by the navigation |
   | `REFSV` | `REFSYS - dt_sv`: receiver clock offset to the satellite clock, `dt_sv` being the satellite clock correction applied to resolve the solution (relativistic correction included) |
   | `MDTR` | troposphere delay applied to this satellite, converted to time (`/ c`) |
   | `MDIO` | modeled ionosphere delay (`/ c`): zero for dual frequency methods (cancelled) |
   | `MSIO` | measured ionosphere delay, when the solver measures it |
   | elevation, azimuth | line of sight |

3. At the **end of each period**, each satellite track is fitted (linear regression,
   evaluated at midtrack: `REFSV`, `SRSV`, `REFSYS`, `SRSYS`, `DSG`, `MDTR`, `SMDT`, `MDIO`,
   `SMDI`) and released as a CGGTTS `Track`, if the satellite was tracked during the whole
   data collection.

A CGGTTS track therefore requires **continuous solutions for ~13 minutes**:
satellites rising or setting during the period, or affected by a data gap, do not produce
a track (counted as `short_tracks` in the report).

**RTK is not supported**: differential navigation does not resolve the clock state.
`QcCggttsProcessor::new` returns `QcCggttsError::RtkNotSupported`.

## Tracking schedule

By default, tracks follow the BIPM schedule (`CommonViewCalendar::bipm()`):

- 16' periods, aligned to MJD 50722 + 2', with a -4' daily offset (GPS sidereal day),
  90 periods per day;
- each period starts with a 3' setup (no data collection), followed by 13' of data collection.

CGGTTS is scheduled and expressed in UTC. The tracks start at the first observation
of the data collection: with 30 s observations sampled on the GPS time grid, a track
starting at 00:09:00 UTC starts at 00:09:12 UTC (GPST - UTC = 18 s in 2020), and lasts
12'30".

## Configuration

`QcCggttsConfig`:

| Field | Default | Description |
|-------|---------|-------------|
| `calendar` | `CommonViewCalendar::bipm()` | Tracking schedule |
| `setup_duration` | 3' | Setup duration at the beginning of each period (must match the calendar) |
| `min_track_duration` | 12' | Minimal track duration |
| `gap_tolerance` | 1' | A larger gap restarts the satellite track |
| `class` | `CommonViewClass::MultiChannel` | Common view class of the tracks (all in view receivers) |
| `pvt_solutions` | `false` | Also stream the P.V.T solutions |

The navigation itself is configured with a `QcNavigationConfig`, see [Navigation](README.md).
CGGTTS files are produced per constellation: navigate with a single constellation
(`with_constellations(&[Constellation::GPS])`) to produce a GPS CGGTTS file.

## Example

[examples/cggtts.rs](../../examples/cggtts.rs)

```rust,no_run
use gnss_qc::prelude::*;

// 1. Context
let mut products = QcInputProducts::default();

for file in [
    "data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz",
    "data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz",
    "data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz",
] {
    products.load_gzip_file(&QcInputConfig::default(), file).unwrap();
}

// 2. Absolute navigation (SPP, CPP or PPP) + CGGTTS post-fit
let navigation = QcNavigationConfig::default()
    .with_method(Method::CPP)
    .with_constellations(&[Constellation::GPS])
    .with_user_parameters(UserParameters::new(UserProfile::Static, ClockProfile::Quartz));

let cggtts = QcCggttsProcessor::new(
    MetaAlmanac::latest().unwrap(),
    navigation,
    QcCggttsConfig::default(),
)
.unwrap();

// 3. Engine
let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(cggtts)
    .build()
    .unwrap();

// 4. Tracks are streamed as soon as their period ends
let mut file = CGGTTS::default();
file.header = file.header.with_station("ESBC").with_reference_frame("ITRF");

for item in engine {
    match item.unwrap().payload {
        QcOutput::CggttsTrack(track) => {
            println!(
                "{} {} REFSYS={:e}s DSG={:e}s",
                track.track.epoch, track.track.sv, track.track.data.refsys, track.track.data.dsg
            );
            file.tracks.push(*track.track);
        },
        QcOutput::CggttsReport(report) => println!("{:?}", report),
        _ => {},
    }
}

file.to_file("ESBC.cggtts").unwrap();
```

```bash
cargo run --release --features cggtts --example cggtts > ESBC.cggtts
```

## Outputs

| Output | Description |
|--------|-------------|
| `QcOutput::CggttsTrack(QcCggttsTrack)` | `receiver` + CGGTTS `Track` (boxed, UTC): satellite, start time, duration, elevation, azimuth, `REFSV`, `SRSV`, `REFSYS`, `SRSYS`, `DSG`, `IOE`, `MDTR`, `SMDT`, `MDIO`, `SMDI`, frequency code |
| `QcOutput::CggttsReport(QcCggttsReport)` | End of session: tracks released, short tracks, fit errors |
| `QcOutput::NavigationReport` | End of session: navigation summary |
| `QcOutput::PVTSolution` | P.V.T solutions, when `pvt_solutions` is set |

Tracks are standard CGGTTS tracks: they can be formatted (`CGGTTS::format`, `to_file`),
parsed back, compared between laboratories, or analyzed further with the CGGTTS library.

Frequency codes (`FRC`):

| Method | GPS / QZSS | Galileo | BeiDou |
|--------|------------|---------|--------|
| SPP | `L1C` | `E1` | `B1i` |
| CPP, PPP | `L3P` | `L3E` | `L3B` |

The issue of ephemeris (`IOE`) is the IODE (GPS, QZSS), IODnav (Galileo) or AODE (BeiDou)
of the broadcast ephemeris selected for the satellite.

## Results

ESBC00DNK, 2020-06-25 00:00-02:00, GPS, NAV + GRG SP3 (verified by the test suite,
`src/tests/navigation/cggtts.rs`):

- 7 tracking periods, 7 to 8 tracks per period (CPP, SPP and PPP);
- `REFSYS` matches the navigation clock offset at midtrack (5 ns);
- `REFSYS - REFSV` matches the satellite clock of the precise products (50 ns, relativistic correction included);
- `MDTR` corresponds to a 2.0-2.8 m zenith troposphere delay;
- `DSG` ~1 ns: the stability of the receiver internal oscillator over 13';
- formatted then parsed back as a CGGTTS 2E file, with the 0.1 ns resolution of the format.

```text
SAT CL  MJD  STTIME TRKL ELV AZTH   REFSV      SRSV     REFSYS    SRSYS  DSG IOE MDTR SMDT MDIO SMDI FR HC FRC CK
             hhmmss  s  .1dg .1dg    .1ns     .1ps/s     .1ns    .1ps/s .1ns     .1ns.1ps/s.1ns.1ps/s
G05 FF 59025 000912  750 562 2165     4962618    -12     4809269    -20   11  12   98    7    0    0  0  0 L3P D5
G07 FF 59025 000912  750 446  676     7931196     44     4809269    -20   11  94  116   15    0    0  0  0 L3P E5
G13 FF 59025 000912  750 520 2788     4597733    -37     4809269    -20   11  71  103  -11    0    0  0  0 L3P 02
```

## Limitations

- Hardware delays (cable, internal, reference delays) are not applied: they are described
  in the CGGTTS header by the application.
- The receiver clock offset is resolved in the navigation timescale (GPST): with several
  constellations, the inter-system biases are not reported per constellation. Produce one
  CGGTTS file per constellation, navigating with that constellation only.
- Antenna phase centers are not applied yet.
