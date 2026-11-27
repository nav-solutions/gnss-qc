Real-time TEC maps from NTRIP streams
=====================================

The [ntrip_tec example](../examples/ntrip_tec/main.rs) (`ntrip` and `ionex` features) is a
ready to use tool: it deploys one NTRIP client per reference station of a network, estimates
the Total Electron Content of every line of sight in real-time, grids the estimates into TEC
maps and publishes them as a standard IONEX file, updated every time a new map is complete.

```text
 NTRIP station 1 ──┐
 NTRIP station 2 ──┤             QcObsTecProcessor         gridding, every
 ..                ├─► QcEngine ───────────────────► TEC ─────────────────► IONEX (one file per
 NTRIP station N ──┤             TecEstimate, TecArc       --interval         UTC day, rewritten)
 NTRIP BCEP      ──┘             (pierce points)                            + console
```

It combines three building blocks of the library:

- [NTRIP clients](ntrip.md) (`QcNtripSource`): one per station, plus optional ephemerides streams,
- [TEC from signal observations](obs_tec.md) (`QcObsTecProcessor`): leveled slant TEC and
  vertical TEC at the pierce points, for every receiver of the network,
- the gridding of the `obs2tec` command of rinex-cli (same algorithm, same IONEX), applied
  continuously.

> [!WARNING]
> **Uncalibrated TEC.** The differential code biases (DCB) of the satellites and of the
> receivers are not corrected: every estimate is biased by a few TECu to tens of TECu.
> See [the limitations](#limitations).

## Usage

```bash
cargo run --release --features ntrip,ionex --example ntrip_tec -- \
    ntrip://user:password@caster.centipede.fr:2101/STATION1 \
    ntrip://user:password@caster.centipede.fr:2101/STATION2 \
    ntrip://user:password@caster.centipede.fr:2101/STATION3 \
    ephemerides=ntrip://user:password@products.igs-ip.net:2101/BCEP00BKG0 \
    --interval "15 min" --grid 40,55,-5,10 --point "Paris:48.85,2.35" \
    --output /var/lib/tec
```

Each argument `ntrip://[user[:password]@]host[:port]/MOUNT` (`ntrips://` for TLS) is one
station of the network. Prefix a stream with `ephemerides=` when it only delivers radio
ephemerides (like `BCEP00BKG0`): the engine never waits for it. Stations of different casters
can be mixed.

| Option | Description |
|--------|-------------|
| `--position MOUNT:X,Y,Z` | ECEF coordinates of a station (m), when its stream does not deliver them (RTCM 1005/1006) |
| `-c`, `--constellations` | Constellations, comma separated (default: `GPS`) |
| `--elevation-mask DEGREES` | Elevation mask (default: 15°) |
| `--shell-altitude KM` | Altitude of the single layer (default: 450 km) |
| `--interval PERIOD` | One map every PERIOD (default: `"15 min"`) |
| `--grid network\|world\|LAT0,LAT1,LON0,LON1` | Extent of the maps (default: `network`, bounding box of the pierce points of the day) |
| `--dlat`, `--dlon` | Grid spacing (default: 2.5° × 5°) |
| `--radius DEGREES` | Influence radius of a pierce point (default: the largest grid spacing) |
| `--min-samples COUNT` | Minimal number of pierce points of a node, empty node otherwise (default: 3) |
| `--no-rotation` | Earth-fixed maps (default: Sun-fixed frame, IONEX specifications) |
| `--point NAME:LAT,LON` | Point of interest, printed map after map (repeat as needed) |
| `--output DIR` | Directory of the IONEX files (default: `.`) |
| `--prefix NAME` | IONEX file name prefix (default: `NTRIPTEC`) |
| `--duration PERIOD` | Stops after PERIOD (default: never, Ctrl+C) |
| `--max-latency PERIOD` | A station lagging behind the others by more than PERIOD is not waited for (default: `"30 s"`) |
| `--reconnect PERIOD` | Reconnection delay after a failure (default: `"10 s"`, doubled after each failure, up to 2 minutes) |
| `--no-reconnect` | No reconnection: a failed station is left out |
| `--reference-epoch EPOCH` | Approximate date of recorded streams (replay, see [time tags](ntrip.md#time-tags)) |

Requirements, per station: dual frequency code **and** phase observations (MSM4 to MSM7),
the coordinates of the station (RTCM 1005/1006, streamed by most reference stations, or
`--position`), and radio ephemerides (from the station streams or from an ephemerides stream).
A station contributes as soon as its coordinates and the ephemerides of its satellites are known.

## Outputs

```text
OUTPUT/
├── NTRIPTEC_20261760000_01D_15M_GIM.INX    yesterday: complete (96 + 1 maps)
└── NTRIPTEC_20261770000_01D_15M_GIM.INX    today: one more map every 15 minutes
```

- One IONEX file per UTC day, named after the IGS long name convention
  (`PREFIX_YYYYDDD0000_01D_15M_GIM.INX`), from the map 00:00 to the map 24:00 of that day.
- The file of the day is **rewritten** every time a map is complete, atomically (written aside,
  then renamed): it is always a valid IONEX, even when the tool is stopped with Ctrl+C. Readers
  (rinex-cli, a web server..) always find a complete file.
- The console prints every new map: receivers, pierce points, nodes with a value, mean TEC, and
  the points of interest:

```text
NTRIPTEC_20201770000_01D_01H_GIM.INX map 2020-06-25T01:00:00 UTC: 2 receivers, 1598 pierce points, 69/99 nodes, mean -4.0 TECu, Esbjerg: -2.8 TECu (uncalibrated)
```

The IONEX files are standard IONEX (mapping function `COSZ`, elevation cutoff, single layer
altitude, number of stations and satellites, comments stating that the maps are uncalibrated):
the [IONEX inputs](ionex.md) of GNSS-Qc, and the `tec` command of rinex-cli, analyze them like any
other IONEX:

```bash
rinex-cli -f OUTPUT/NTRIPTEC_20261770000_01D_15M_GIM.INX tec --region "France:41,51,-5,10"
```

## Real-time processing

### Map after map

A map `Tk` gathers the pierce points of `[Tk − interval/2, Tk + interval/2[` (round UTC epochs).
It is complete, then published, as soon as the network moved past `Tk + interval/2`: the latency
of a map is half the interval, plus the latency of the streams.

### Leveling: running, then exact

In post processing, the phase of an arc is leveled onto the code with the mean of the whole arc.
In real-time, the arc is still being observed: `QcObsTecProcessor` streams each estimate with the
**running** leveling (mean of the arc so far), then the **exact** leveling once the arc ended
(`QcTecArc`: loss of lock, data gap, cycle slip or the satellite setting).

The tool applies the exact leveling as soon as it is known: since the file of the day is rewritten
with every new map, the earlier maps of the day are **refined** with the exact leveling of the arcs
that ended in the meantime. The first estimates of a rising satellite (few samples, low elevation)
are the least accurate when the map is first published.

### Grid

With `--grid network` (default), the grid is the bounding box of the pierce points of the day:
it may grow during the day (rising satellites, new stations), and every rewrite of the file uses
the latest grid. Operational services should rather use a fixed grid (`--grid LAT0,LAT1,LON0,LON1`
or `--grid world`), so consumers always get the same nodes.

### Day after day

When the first map of a new UTC day is complete, the file of the previous day is updated one last
time (exact leveling), then its estimates are released: the memory holds at most about one day
of estimates (10 stations, 10 satellites, 30 s: about 300 000 estimates, a few tens of MB),
and does not grow from one day to the next. 1 Hz streams hold 30 times more.

## Network: what to watch for

**Synchronization.** The engine processes an epoch once every *live* station moved past it, so
that the stations of an epoch are presented together. A station that lags behind the others by
more than `--max-latency` (silent, reconnecting, slow link) is no longer waited for: the network
goes on without it, its late data is dropped until it catches up. Every client reconnects
automatically (`--reconnect`), and a station that cannot be used (unknown mount point, refused
credentials) is left out without stopping the others ([real-time networks](ntrip.md#real-time-networks)).

**Monitoring.** The state of every station is reported every minute: connection, number of
connections, last failure, decoded messages (MSM, legacy observations, ephemerides,
coordinates), receiver and antenna (1033), TEC estimates. A station without TEC estimate comes with
a hint: no observation message, unknown coordinates (`--position`), no radio ephemeris at all
(add an `ephemerides=` stream, the Centipede streams do not always carry them).

**Legacy and MSM.** Centipede stations stream their observations twice (legacy 1004/1012 and
MSM7): the legacy messages are skipped once MSM were decoded. Glonass and SBAS are decoded, but
not used for the TEC (Glonass: FDMA, SBAS: no orbits).

**Signals.** Each station streams the signals of its receiver: C1C or C1W, C2W, C2L or C5Q..
`QcObsTecProcessor` selects the best pair of each satellite, so two stations may estimate the TEC
of the same satellite with different signal pairs, which have different biases. Prefer one
constellation (`-c GPS`), and check the observables of the stations (MSM4 to MSM7, signal masks).
Glonass MSM messages are not decoded yet.

**Coordinates.** The pierce points depend on the position of each station: it must be known
(RTCM 1005/1006 or `--position`). Until then, the station does not contribute.

**Coverage.** A station covers a disk of about 25° (15° elevation mask, 450 km shell). Nodes
without enough pierce points (`--min-samples`) remain empty (`9999`), and a regional network
leaves most of a world grid empty.

## Limitations

The maps are **uncalibrated**: the differential code biases are not corrected (see
[TEC from signal observations](obs_tec.md)):

- every receiver has its own DCB, up to tens of ns (2.85 TECu per ns, GPS L1/L2): neighbor stations
  disagree by a constant (ESBC00DNK and MOJN00DNK, 65 km apart: about 19 TECu along the line of
  sight, ±6 TECu around the mean in vertical), and the gridding averages estimates biased differently,
- every satellite has its own DCB (a few ns): the value of a node changes when a satellite rises or
  sets, more visibly with short intervals (15 min) that gather few satellites per node,
- negative TEC values are common, especially at the edges of the network.

These maps are suited to the relative variations of the ionosphere (day / night cycle,
disturbances, traveling ionospheric disturbances), not to absolute TEC. Correcting the satellite
DCB (Bias SINEX) and estimating the DCB of every station is the next step.

Other limitations: single layer model (inaccurate at low elevations), inverse distance weighting
(no extrapolation, no smoothing in time).

## Validation

- `src/tests/rtcm/ntrip.rs` (`ntrip_obs_tec`): ESBC00DNK and MOJN00DNK (2020-06-25) encoded as
  RTCM 3 (MSM7, station coordinates 1005, radio ephemerides streamed by ESBC) and served by two
  local NTRIP casters, GPS, one hour: the 1693 streamed TEC estimates are those of the RINEX files
  (same lines of sight, same signals), within 0.003 TECu (RTCM resolution).
- The tests of the example (`cargo test --release --features ntrip,ionex --example ntrip_tec`):
  gridding, Sun-fixed rotation, exact leveling, daily windows, map publication, IONEX round trip.
- Replay of 3 hours of both stations (one map every hour, `--grid 45,65,-15,35`): the IONEX file is
  read back by `rinex-cli tec`. Mean TEC over Denmark: −0.9 TECu (00:00) and −3.7 TECu (01:00),
  against −0.9 and −3.55 TECu for `rinex-cli obs2tec` on the RINEX files of the whole day (the
  phase arcs of the replay are shorter, so their leveling slightly differs).

To replay recorded streams (RTCM captures), serve them with a local caster and give their date:

```bash
cargo run --release --features ntrip,ionex --example ntrip_tec -- \
    ntrip://127.0.0.1:2101/ESBC00DNK0 ntrip://127.0.0.1:2101/MOJN00DNK0 \
    --reference-epoch "2020-06-25T00:00:00 GPST" --interval "1 h" --output /tmp/tec
```
