RTCM streams and NTRIP clients
==============================

GNSS-Qc accepts real-time RTCM 3 streams as inputs of the MISO engine, next to (or instead of)
files. The main application is the NTRIP client (`ntrip` feature): each client connects to one
mount point of a caster and streams it into the engine. Deploy as many clients as needed:

```text
 NTRIP rover (mount A) ──┐
 NTRIP base  (mount B) ──┤                ┌─ signals, coverage, residuals..
 NTRIP BCEP  (mount C) ──┼──► QcEngine ───┼─ P.V.T solutions (SPP, CPP, PPP)
 SP3 / CLK files      ───┘                └─ RTK (rover / base)
```

| Feature | Provides |
|---------|----------|
| `rtcm` | `QcRtcmDecoder`: RTCM 3 messages to data points, without any I/O (files, serial ports, sockets..). `QcRtcmSource`: recorded RTCM streams. |
| `ntrip` | `QcNtripSource`: NTRIP clients, powered by [ntrip-client](https://github.com/nav-solutions/ntrip-client) (implies `rtcm`, adds the Tokio runtime) |

- [What a stream contributes](#what-a-stream-contributes)
- [Indexing and roles](#indexing-and-roles)
- [NTRIP clients](#ntrip-clients)
- [RTK with two NTRIP clients](#rtk-with-two-ntrip-clients)
- [Time tags](#time-tags)
- [Recorded streams](#recorded-streams)
- [Validation](#validation)
- [Limitations](#limitations)

## What a stream contributes

A stream plays the role of an Observation RINEX and of a Navigation RINEX at the same time,
so it contributes to every processor of the engine (signals, ephemerides, coverage, residuals,
navigation, RTK..):

| RTCM messages | Constellation | Data points |
|---------------|---------------|-------------|
| 1074 - 1077 (MSM4 - MSM7) | GPS | Signals: pseudo range (`C1C`, m), phase (`L1C`, cycles), doppler (`D1C`, Hz, MSM5/7), C/N0 (`S1C`, dB-Hz), loss of lock (lock time indicators) |
| 1084 - 1087 | Glonass | idem, FDMA: phases in cycles of the frequency channel of each satellite (MSM5/7, 1020 or 1012) |
| 1094 - 1097 | Galileo | idem |
| 1104 - 1107 | SBAS | idem (MSM satellite 1: PRN 120, `S20`) |
| 1114 - 1117 | QZSS | idem |
| 1124 - 1127 | BeiDou | idem |
| 1004 (legacy L1/L2) | GPS | Signals `C1C`/`C1P`, `L1C`/`L1P`, `C2X`/`C2P`/`C2D`/`C2W` (L2 code indicator), phases, C/N0, when the stream has no GPS MSM |
| 1012 (legacy L1/L2) | Glonass | Signals `C1C`/`C1P`, `C2C`/`C2P`, phases (frequency channel), C/N0, when the stream has no Glonass MSM |
| 1019 | GPS | Radio ephemeris |
| 1020 | Glonass | Radio ephemeris: PZ-90 state vector (`satPosX`.. km, `velX`.. km/s, `accelX`.. km/s²), `channel`, clock (−τn, γn) |
| 1045 (F/NAV), 1046 (I/NAV) | Galileo | Radio ephemeris |
| 1042 | BeiDou | Radio ephemeris |
| 1044 | QZSS | Radio ephemeris |
| 1005, 1006 | - | Coordinates of the station (antenna reference point) |
| 1008, 1033 | - | Antenna and receiver descriptors (`QcRtcmStats::antenna`, `QcRtcmStats::receiver`) |
| 1230 | Glonass | Code-phase biases of the station (`QcRtcmStats::glonass_code_phase_biases`) |

Signals and ephemerides are expressed exactly like their RINEX counterparts (RINEX observables
and units, RINEX orbital fields in radians), so the processors cannot tell a stream from a file.
The coordinates of the station (1005/1006) update the reference position of the source
(`QcPayload::ReferencePosition`): this is how the position of an RTK base is learned in real-time.

Other messages (SSR corrections, network RTK, legacy L1 only observations 1001-1003, 1009-1011..)
are counted (`QcRtcmStats::ignored`) but not interpreted.

Many reference stations (the Centipede network, for example) stream the same observations
twice: legacy messages (1004, 1012) for older rovers, and MSM. Once MSM observations of a
constellation were decoded, its legacy messages are skipped (`QcRtcmStats::redundant`), and an
observation is never delivered twice for the same epoch.

Glonass signals are frequency multiplexed: until the frequency channel of a satellite is known
(MSM5/7 extended information, 1020 or 1012), only its pseudo ranges and C/N0 are delivered.
Glonass ephemerides are decoded and stored, but the orbits of Glonass satellites are not
computed by GNSS-Qc yet (no Glonass navigation, TEC or signal combinations).

## Indexing and roles

Indexing is how the engine identifies each source (see [context](context.md)). A mount point
streams the data of a single station, so a stream is indexed like a file: by default, by its
mount point name (`QcIndexing::Custom("MOUNT")`), or by any `QcIndexing` of your choice.
The identity of a signal source must be unique in the engine.

Each client declares its role (`QcNtripRole`), that is to say what it contributes to:

| Role | Signals | Ephemerides | Station coordinates | The engine waits for it |
|------|---------|-------------|---------------------|-------------------------|
| `Rover` (default) | ✓ | ✓ (when streamed) | | ✓ |
| `Base` | ✓ | ✓ (when streamed) | ✓ (1005/1006, or configured) | ✓ |
| `Ephemerides` | | ✓ | | never |

The role matters: signal sources drive the processing (an epoch is processed once every signal
source moved past it), while an ephemerides stream (like `BCEP00BKG0`) must never be waited for.
The navigation configuration may also be deduced from the roles
(`QcNavigationConfig::with_ntrip_roles`): first rover, first base for RTK.

## NTRIP clients

Each client has its own connection settings (`QcNtripConfig`): caster, port, TLS, mount point,
credentials, indexing, role, and optionally the position of the station. They are usually parsed
from a URL, `ntrip://[user[:password]@]host[:port]/MOUNT` (`ntrips://` for TLS, default port: 2101):

```rust,no_run
use gnss_qc::prelude::*;

let rover = "ntrip://user:password@caster.centipede.fr:2101/ROVER"
    .parse::<QcNtripConfig>()
    .unwrap()
    .with_role(QcNtripRole::Rover);

let ephemerides = QcNtripConfig::new("products.igs-ip.net", 2101, "BCEP00BKG0")
    .with_credentials("user", "password")
    .with_role(QcNtripRole::Ephemerides);

let mut engine = QcEngine::<QcOutput>::builder()
    .with_source(QcNtripSource::new(rover).unwrap())
    .with_source(QcNtripSource::new(ephemerides).unwrap())
    .with_processor(QcSignalProcessor)
    .build()
    .unwrap();

// event loop: the engine never blocks
loop {
    match engine.poll_next() {
        Ok(QcPoll::Ready(result)) => { /* forward the result */ },
        Ok(QcPoll::Pending) => std::thread::sleep(std::time::Duration::from_millis(20)),
        Ok(QcPoll::EndOfStream) => break, // all streams closed
        Err(e) => panic!("{}", e), // connection failure..
    }
}
```

Each client runs in its own background thread, with its own (single threaded) Tokio runtime:
the engine itself remains runtime agnostic and is never blocked by a client (`QcPoll::Pending`
while waiting for data). Connection failures (unknown mount point, invalid credentials,
unreachable caster) are reported by the engine as source errors
(`NTRIP connection failure: caster response "HTTP/1.0 401 Unauthorized"`). A client stops when
its `QcNtripSource` is dropped, or when the caster closes the connection (end of stream).
`QcNtripSource::stats` reports what was received so far (`QcRtcmStats`).

The [ntrip example](../examples/ntrip.rs) deploys one client per argument:

```bash
cargo run --release --features ntrip,navigation --example ntrip -- \
    rover=ntrip://user:password@caster:2101/ROVER \
    base=ntrip://user:password@caster:2101/BASE
```

For real-time TEC maps of a network of stations (one client per station, IONEX output),
see [real-time TEC maps from NTRIP streams](ntrip_tec.md).

## RTK with two NTRIP clients

Rover and base are two signal sources, synchronized epoch by epoch by the engine. The base
stream usually delivers its coordinates (1005/1006) and the radio ephemerides:

```rust,ignore
use gnss_qc::prelude::*;

let streams = [
    "ntrip://user:password@caster:2101/ROVER"
        .parse::<QcNtripConfig>()?
        .with_role(QcNtripRole::Rover),
    "ntrip://user:password@caster:2101/BASE"
        .parse::<QcNtripConfig>()?
        .with_role(QcNtripRole::Base),
];

// rover: ROVER, RTK base: BASE (position from its stream)
let cfg = QcNavigationConfig::default()
    .with_method(Method::CPP)
    .with_ntrip_roles(&streams);

let mut builder = QcEngine::<QcOutput>::builder()
    .with_processor(QcNavigationProcessor::new(almanac, cfg)?);

for stream in streams {
    builder = builder.with_source(QcNtripSource::new(stream)?);
}
```

Precise products (SP3, Clock RINEX files) may be added as other sources, like in post processing.

## Real-time networks

A network of stations (many NTRIP clients deployed together) needs three settings, so that one
station never holds or stops the other ones:

```rust,ignore
use std::time::Duration as StdDuration;
use gnss_qc::prelude::*;

let mut builder = QcEngine::<QcOutput>::builder()
    // a station lagging behind the others by more than 30 s is not waited for
    .with_max_latency(Duration::from_seconds(30.0))
    // a station that fails (unknown mount point..) is ended, the others go on
    .with_fault_tolerance(true);

let mut monitors = Vec::new();

for mount in ["STATION1", "STATION2", "STATION3"] {
    let stream = QcNtripConfig::new("caster.centipede.fr", 2101, mount)
        // reconnects after a failure: 10 s, doubled after each failure, up to 2 minutes
        .with_reconnection(StdDuration::from_secs(10));

    let source = QcNtripSource::new(stream)?;
    monitors.push(source.monitor()); // connection state, decoded messages
    builder = builder.with_source(source);
}
```

- `QcNtripConfig::with_reconnection`: the client reconnects after a connection failure, or when the
  caster closes the connection, forever (default: no reconnection, failures are reported).
- `QcEngineBuilder::with_max_latency`: the engine waits for every live station, so that the
  stations of an epoch are processed together. A station that lags behind the newest data of the
  session by more than this latency (silent, reconnecting..) is no longer waited for: its late data
  is dropped (`QcEngineStats::late_items`) until it catches up (`QcEngineStats::incomplete_epochs`
  counts the epochs processed without it). Replays at accelerated speed need a large latency
  (the streams do not progress at the same pace).
- `QcEngineBuilder::with_fault_tolerance`: a source that fails (unknown mount point, refused
  credentials) is ended and the session goes on (`QcEngineStats::failed_sources`). Default: the
  failure of a source aborts the session.
- `QcNtripSource::monitor`: a handle on the state of a client (connected, number of connections,
  last failure, `QcRtcmStats`), that remains available once the source is moved into the engine.

The [real-time TEC example](ntrip_tec.md) deploys these settings.

## Time tags

MSM messages only carry the time of week (time of day for Glonass). The week is resolved against
a reference epoch: the system time by default (live streams), or the approximate date of the
stream (`QcNtripConfig::with_reference_epoch`, `QcRtcmDecoder::with_reference_epoch`) to replay
recorded data. Then the reference follows the stream. All signals are time tagged in GPST
(BeiDou time tags are shifted by the 14 s BDT − GPST offset), so the observations of all
constellations of one epoch share the same time tag, and the epochs of two stations
(rover / base) match exactly.

Glonass time tags (time of day, Moscow time UTC(SU) + 3 h) are converted to GPST with the leap
seconds, so that Glonass observations share the time tag of the other constellations.

Radio ephemerides and station coordinates are time tagged with their *reception* time
(time tag of the last observation of the stream): each source must remain time ordered,
while the Time of Clock of an ephemeris may lie in the future of the stream. The ephemerides and
coordinates received before the first observation of a live stream are held until that
observation, then tagged with its epoch: the system time is ahead of the observations (latency
of the stream), so tagging them with the system time would make the stream look disordered
(`source is not time ordered` warnings). A stream without observations (ephemerides only)
releases them after 10 s, tagged with the system time.

The `source is not time ordered` warning is not fatal: the observations are reordered by the
engine, as long as their epoch was not processed yet (otherwise they are dropped and counted in
`QcEngineStats::late_items`).

## Recorded streams

`QcRtcmSource` processes recorded RTCM bytes (a file, a capture of a stream), with the same
decoder. `QcRtcmDecoder::decode_bytes` decodes a byte stream (frames are detected by their
preamble and CRC, a truncated frame is left for the next read), `QcRtcmDecoder::decode` decodes
messages that were already framed (by the NTRIP client, for example).

```rust,ignore
use gnss_qc::prelude::*;

let bytes = std::fs::read("capture.rtcm3")?;

let source = QcRtcmSource::new(
    QcSourceDescriptor::new(QcIndexing::Custom("ROVER".to_string()))
        .with_signals()
        .with_ephemerides(),
    QcRtcmDecoder::new().with_reference_epoch(Epoch::from_str("2020-06-25T00:00:00 GPST")?),
    bytes,
);
```

## Validation

The test suite encodes real RINEX data (ESBC00DNK and MOJN00DNK, 2020-06-25) as RTCM 3 frames
(MSM7 for GPS, Galileo, BeiDou and QZSS, ephemerides 1019, 1042, 1044, 1045, 1046, coordinates 1005),
and streams them through local NTRIP casters:

- MSM7: every observation that MSM can carry is decoded (6536 pseudo ranges, 5852 phases,
  5865 dopplers, 6536 C/N0 over 30 minutes): pseudo ranges within 1 mm, phases within the integer
  number of cycles of the code alignment, dopplers and C/N0 within the RTCM resolution;
- ephemerides (2096 frames, four constellations): satellite positions within 5 cm, clocks within 0.1 ns;
- week resolution (rollovers of the time of week and of the truncated week counters);
- connection failures (credentials, unknown mount point, unreachable caster);
- Glonass (MSM7 with frequency channels and Glonass time) and SBAS (MSM7): every observation is
  decoded, like the other constellations; legacy observations (1004, 1012): pseudo ranges within
  the 2 cm resolution, phases within the integer number of cycles of the alignment;
  Glonass ephemerides (1020): state vectors within the RTCM resolution; descriptors (1008, 1033)
  and Glonass biases (1230);
- legacy and MSM observations streamed together: no duplicate;
- a network of NTRIP clients: a station refused once by its caster ("empty response") reconnects,
  a silent station is not waited for (maximal latency), an unknown mount point is ended (fault
  tolerance), and the session goes on with the other stations;
- RTK-CPP with two NTRIP clients (rover ESBC, base MOJN, 65 km, base coordinates and ephemerides
  from the base stream): 121 solutions over one hour, within 1 mm of the solutions obtained from
  the RINEX files.

## Limitations

- SSR corrections and legacy L1 only observations (1001-1003, 1009-1011) are not interpreted yet.
- Glonass orbits are not computed yet (the ephemerides are decoded and stored).
- A client does not send its position to the caster (`GGA` sentences): virtual reference
  station (VRS) networks are not supported yet.
