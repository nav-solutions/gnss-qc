TEC from signal observations
============================

The `QcObsTecProcessor` (`ionex` feature) estimates the Total Electron Content from the signal
observations of every receiver (Observation RINEX, CRINEX, RTCM / NTRIP streams..), at the
Ionospheric Pierce Point (IPP) of each line of sight. It works on any number of receivers:
deploy it on a network of stations to cover a region.

GNSS-Qc does not build TEC maps: each estimate is a **point** (epoch, receiver, satellite,
pierce point, slant TEC, vertical TEC), serialized like any other output. Collecting the points
into a grid (IONEX), with the interpolation and the time resolution of your choice, is the job
of the application (see [Building TEC maps](#building-tec-maps-ionex)).

- [Principle](#principle)
- [Where is the TEC estimated?](#where-is-the-tec-estimated)
- [Differential code biases](#differential-code-biases)
- [Configuration](#configuration)
- [Outputs](#outputs)
- [Building TEC maps (IONEX)](#building-tec-maps-ionex)
- [Validation](#validation)

## Principle

For each satellite, two frequencies are selected: the lowest band (L1, E1, B1I..) and a second
band (GPS: L2 then L5, Galileo: E5a then E5b, E5, E6, BeiDou: B3I then B2..), with a code and
a phase on each, precise codes first (`W`, `P`, `Y`, then `C`, `X`..).

The ionosphere delays the codes and advances the phases by `I = 40.3 TEC / f²`. The
ionospheric delay difference between the two frequencies gives the slant TEC (STEC), with:

```text
k = f1² fj² / (40.3 (f1² − fj²))        [el/m² per meter]     1 TECu = 10^16 el/m²

STEC(code)  = k (Cj − C1)               absolute, but noisy (code noise, multipath)
STEC(phase) = k (L1 − Lj)               L in meters: precise, but ambiguous
```

For GPS L1/L2, `k` = 9.52 TECu per meter (7.76 TECu per meter for Galileo E1/E5a).

The phase ambiguities (an unknown number of cycles on each frequency) are constant as long
as the phases are tracked without interruption. The phase is therefore **leveled** on the code,
over each continuous phase arc:

```text
STEC = STEC(phase) + < STEC(code) − STEC(phase) >arc − k c (DCB(sat) + DCB(rx))
```

The last term (differential code biases) is null for now, see
[Differential code biases](#differential-code-biases).

The mean over the arc is a running mean (causal, compatible with real-time streams): the first
samples of an arc are less accurate. When the arc ends (loss of lock, data gap, cycle slip,
end of stream), its final mean is emitted (`QcOutput::TecArc`): `phase_slant_tecu +
leveling_tecu` (of the arc) gives the exact leveling of every sample of the arc, in post
processing. An arc ends:

- on a loss of lock (LLI) of one of the two phases,
- after a data gap longer than `max_gap` (10 minutes),
- on a jump of the Geometry Free phase larger than `slip_threshold_m` (0.1 m) between two
  epochs (cycle slip: one L1 cycle is 0.19 m).

Glonass (FDMA: each satellite has its own frequencies) is not supported.

## Where is the TEC estimated?

The slant TEC integrates the electrons along the line of sight, from the satellite to the
receiver. The *single layer model* (thin shell model, as IONEX) concentrates the ionosphere
in a thin shell at a fixed altitude H (450 km by default, `shell_altitude_km`): the TEC is
located where the line of sight pierces the shell, the **Ionospheric Pierce Point**.

```text
              satellite
                 *
                  \
   ----------------x---------------  ionospheric shell, H = 450 km
                    \  z'                x: pierce point (latitude, longitude)
                     \
                      \ z
   ====================o=============  receiver (R = 6371 km)
```

The pierce point depends on the position of the receiver (its reference position, or
`receiver_position_ecef_m`) and on the elevation and azimuth of the satellite (radio
ephemerides or precise orbits, `orbits`). At 15° of elevation it lies about 1300 km away from
the receiver, at 90° right above it: a single receiver probes a disk of about 25° of diameter
around it, a network of receivers covers a region.

The vertical TEC is obtained with the obliquity factor (mapping function) of the single layer
model, z being the zenith angle at the receiver and z' at the pierce point:

```text
sin(z') = R / (R + H) sin(z)        VTEC = STEC cos(z')
```

Both are emitted: the vertical TEC is what IONEX maps contain, the slant TEC is the actual
observation (with the obliquity factor, any other mapping function can be applied).

## Differential code biases

The codes of the two frequencies do not experience the same hardware delays, in the satellite
and in the receiver: the differential code biases (DCB) add to the code ionospheric delay
difference, hence to the leveled TEC: 2.85 TECu per ns for GPS L1/L2. They are of a few ns
(satellites) to tens of ns (receivers).

They are **not corrected yet** (null term): the estimates are *uncalibrated*, biased by
`k c (DCB(sat) + DCB(rx))`, constant per satellite and receiver. The TEC variations (in time,
in space) are accurate, the absolute level is not: negative values are possible. GNSS-Qc will
accept DCB products in the future (Bias SINEX, group delays of the radio ephemerides,
receiver calibrations): the processor will then remove them.

## Configuration

```rust
use gnss_qc::prelude::*;

// default: every receiver, every constellation (but Glonass), elevation mask 15°
let all = QcObsTecProcessor::new(QcObsTecConfig::default());

// GPS only, 30° of elevation, one receiver
let gps = QcObsTecProcessor::new(QcObsTecConfig {
    elevation_mask_deg: 30.0,
    ..QcObsTecConfig::default()
        .with_constellations(&[Constellation::GPS])
        .with_receiver(QcIndexing::Receiver("SEPT POLARX5 #3047937".to_string()))
});
```

| Field | Default | Description |
|-------|---------|-------------|
| `orbits` | `Auto` | Orbital products: precise whenever available, broadcast otherwise |
| `elevation_mask_deg` | 15° | Elevation mask (mapping function errors grow at low elevation) |
| `shell_altitude_km` | 450 km | Altitude of the ionospheric shell (IONEX) |
| `earth_radius_km` | 6371 km | Radius of the Earth (IONEX) |
| `receivers` | all | Receivers to process |
| `constellations` | all | Constellations to process (Glonass excluded) |
| `receiver_position_ecef_m` | reference position | Position of the receiver. Receivers without position are not processed |
| `max_gap` | 10 min | A longer data gap ends the phase arc |
| `slip_threshold_m` | 0.1 m | A larger jump of the Geometry Free phase ends the phase arc |

The receivers need a position (RINEX header, RTCM 1005/1006..) and the satellites an orbit
(radio ephemerides or precise products).

## Outputs

`QcOutput::TecEstimate(QcTecEstimate)`, for each epoch, receiver and satellite:

| Field | Description |
|-------|-------------|
| `receiver`, `sv` | Receiver and satellite |
| `code_1`, `code_j`, `phase_1`, `phase_j` | Observables (`C1W`, `C2W`, `L1C`, `L2W`) |
| `arc`, `arc_samples` | Phase arc, number of samples of the arc so far |
| `elevation_deg`, `azimuth_deg` | Line of sight, at the receiver |
| `latitude_deg`, `longitude_deg`, `altitude_km` | Pierce point |
| `obliquity_factor` | STEC / VTEC |
| `code_slant_tecu` | `k (Cj − C1)` |
| `phase_slant_tecu` | `k (L1 − Lj)` (ambiguous) |
| `leveling_tecu` | Running mean of `STEC(code) − STEC(phase)` over the arc |
| `slant_tecu` | `phase_slant_tecu + leveling_tecu` |
| `vertical_tecu` | `slant_tecu / obliquity_factor` |
| `map_vertical_tecu` | Vertical TEC of the TEC maps (IONEX inputs) at the same pierce point, when available |

`QcOutput::TecArc(QcTecArc)`, when an arc ends: receiver, satellite, observables, arc number,
start, end, samples, `leveling_tecu` (final mean: exact leveling) and `code_noise_tecu`
(dispersion of code − phase: code noise and multipath).

```rust,no_run
use gnss_qc::prelude::*;

let engine = QcEngine::<QcOutput>::builder()
    // .with_products(products) or .with_source(stream): every receiver of the network
    .with_processor(QcObsTecProcessor::new(QcObsTecConfig::default()))
    .build()
    .unwrap();

for item in engine {
    let item = item.unwrap();
    if let QcOutput::TecEstimate(e) = item.payload {
        println!(
            "{} {} {} IPP({:.2}°, {:.2}°) STEC {:.2} VTEC {:.2} TECu",
            item.epoch, e.receiver, e.sv, e.latitude_deg, e.longitude_deg,
            e.slant_tecu, e.vertical_tecu,
        );
    }
}
```

See also the [obs_tec example](../examples/obs_tec.rs) (CSV output), and the
[ntrip_tec example](ntrip_tec.md): real-time TEC maps of a network of NTRIP streams, as IONEX.

## Building TEC maps (IONEX)

An IONEX file is a series of maps (one every `INTERVAL`, often 1 h or 2 h), each a grid of
vertical TEC values (latitude, longitude, at the shell altitude). Building one from the points:

1. **Time**: gather the estimates of each map interval (for example ±30 min around each map
   epoch). The ionosphere follows the Sun: to combine points of different epochs, express their
   longitudes in a Sun-fixed frame (longitude + 15°/h × (t − t_map)), as the
   `RotatedMaps` interpolation of IONEX does.
2. **Space**: for each node of the grid (`DLAT`, `DLON`), combine the vertical TEC of the pierce
   points around it: mean of the points of the cell, inverse distance weighting, or a fit
   (spherical harmonics, as the IGS analysis centers). Nodes without pierce points around them
   remain unknown (`9999` in IONEX).
3. **Calibration**: as long as the differential code biases are not corrected, the maps are
   uncalibrated. The analysis centers estimate the biases together with the maps (one bias per
   satellite and per receiver, constant over the day): that is the next step.

A single station covers a disk of about 25°, a regional network (EUREF, a national network) a
continent, the IGS network the world.

## Validation

ESBC00DNK and MOJN00DNK (Septentrio PolaRx5, 65 km apart), 2020-06-25, GPS L1/L2, broadcast
ephemerides (see `src/tests/obs_tec.rs`):

- 22 141 (ESBC) and 22 133 (MOJN) estimates, pierce points within 15° of Denmark;
- code noise of the leveling: 2.5 TECu (median per arc);
- every arc is closed once, its final leveling equals the running leveling of its last sample;
- the slant TEC of one satellite seen by both receivers (pierce points about 50 km apart)
  differs by a constant, −19 TECu, identical for all satellites, within 0.4 TECu (median
  dispersion): this constant is the difference of the differential code biases of the two
  receivers (6.7 ns). The ionosphere itself is estimated consistently by the two receivers;
- uncalibrated median vertical TEC: +5.8 TECu (ESBC), −5.8 TECu (MOJN): the receiver biases
  shift the absolute level, as expected.
