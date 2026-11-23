MISO engine
===========

[← Context definition](context.md) · [README](../README.md) · [Processors →](processors.md)

The `QcEngine` is a **Multiple Inputs, Single Output** processor:
any number of sources, of any nature, synchronized together, feed the processors,
which produce a single stream of results.

```text
                   ┌──────────────────── QcEngine ─────────────────────┐
 QcSource ──poll──►│ signals FIFO (per source)                         │
 QcSource ──poll──►│ ephemerides  (per source, per satellite)          │──► processors ──► output queue ──► poll_next()
 QcSource ──poll──►│ precise states (per source, per satellite)        │                                  Iterator
                   │ static data (ANTEX)                               │                                  run(sink)
                   └───────────────────────────────────────────────────┘
```

Contents:

- [Sources](#sources)
- [Synchronization](#synchronization)
- [Building and running the engine](#building-and-running-the-engine)
- [Outputs](#outputs)
- [Streaming files](#streaming-files)
- [Real-time operation](#real-time-operation)
- [Memory and performance](#memory-and-performance)

## Sources

A source implements the `QcSource` trait:

```rust,ignore
pub trait QcSource {
    /// Identity and nature of this source
    fn descriptor(&self) -> &QcSourceDescriptor;

    /// Pulls the next data point, without blocking
    fn poll(&mut self) -> Result<QcPoll<QcInputItem>, QcSourceError>;
}
```

`poll` returns:

- `QcPoll::Ready(item)`: one data point;
- `QcPoll::Pending`: nothing available *yet*, the stream is alive (real-time sources only,
  files are never pending);
- `QcPoll::EndOfStream`: this source is over.

Data points (`QcInputItem`) are time tagged and **must be delivered in chronological
order** by each source:

| Payload | Time tag | Content |
|---------|----------|---------|
| `QcPayload::Signal` | sampling epoch | `SignalObservation` (satellite, observable, value, LLI, SNR) + epoch flag |
| `QcPayload::ReceiverClock` | sampling epoch | receiver clock state |
| `QcPayload::Ephemeris` | Time of Clock | radio ephemeris frame |
| `QcPayload::PreciseState` | state epoch | precise position (km) and clock (s) |

Any source may deliver any payload: a real-time RTCM stream carries both observations and
ephemerides. The `QcSourceDescriptor` tells the engine what to expect from a source, so it
knows which sources it must wait for:

```rust,no_run
use gnss_qc::prelude::*;

let descriptor = QcSourceDescriptor::new(QcIndexing::Custom("rtcm".to_string()))
    .with_signals()
    .with_ephemerides()
    .with_reference_position_ecef_m((3582105.291, 532589.731, 5232754.805));
```

### Built-in sources

| Source | Description |
|--------|-------------|
| `QcRinexObsSource` | Observation RINEX / CRINEX (receiver clock, then signals, epoch after epoch) |
| `QcRinexNavSource` | Navigation RINEX (ephemeris frames) |
| `QcSp3Source` | SP3 precise products |
| `QcIterSource` | Any `Iterator<Item = QcInputItem>` (in-memory datasets, adapters) |
| `QcChannelSource` | Real-time: `std::sync::mpsc` channel fed by another thread |
| `QcChainSource` | Concatenates sources that follow each other in time |
| `QcLazySource` | Opens a source (e.g. parses a file) when the engine first needs it |

## Synchronization

### Epoch driven processing

**Signal observations drive the processing**. Epochs are processed in chronological order,
over the union of all signal sources: they do not need to be aligned, nor to cover the
same period. Before an epoch `t` is processed:

1. **completeness**: every live signal source must have moved past `t` (or ended).
   Two receivers sampling the same epoch are presented together. In real-time, this is the
   only condition the engine waits for;
2. **context**: every live context source (ephemerides, precise states) is pulled until
   it covers `t + lookahead`. A pending context source never blocks the processing;
3. **pruning**: expired context data points are dropped;
4. the epoch is presented to every processor (`QcEpochContext`).

Observations that arrive after their epoch was processed are dropped
(`QcEngineStats::late_items`).

When no source provides signals (a navigation file alone for example), the epochs are driven
by the context data points themselves (*context only session*).

### Per satellite buffers

Context data is buffered **per source and per satellite**, around the epoch being processed:

| Store | Selection at `t` | Retention |
|-------|------------------|-----------|
| `QcEphemerisStore` | valid ephemeris (validity period of its constellation) closest to `t`, from any source (`select`) or from a given agency (`select_from`) | until its validity period expires |
| `QcPreciseStore` | Lagrange interpolation, order 10 (position, velocity, clock), from any source (`interpolate`) or a given agency (`interpolate_from`) | 3 hours |

Precise products are never extrapolated, nor interpolated over a data gap. The default
lookahead (3 hours) and retention (3 hours) allow interpolation up to the very beginning
and end of the products (order 10 at 15' sampling spans 2h30).

```rust,no_run
use gnss_qc::prelude::*;

fn inspect(ctx: &QcEpochContext) {
    for rx in ctx.receivers {
        for signal in rx.signals.iter() {
            if let Some((agency, eph)) = ctx.ephemerides.select(signal.sv, ctx.epoch) {
                println!("{} {} covered by {} (toc={})", ctx.epoch, signal.sv, agency, eph.toc);
            }

            if let Some((agency, state)) = ctx.precise_states.interpolate(signal.sv, ctx.epoch) {
                println!("{} {} precise position {:?} km", ctx.epoch, signal.sv, state.position_km);
            }
        }
    }
}
```

## Building and running the engine

```rust,no_run
use gnss_qc::prelude::*;

let products = QcInputProducts::default();

let t0 = Epoch::from_gregorian_utc_at_midnight(2020, 6, 25);

let engine = QcEngine::<QcOutput>::builder()
    // sources: products table, and/or any QcSource
    .with_products(products)
    // processors run in insertion order
    .with_processor(QcSignalProcessor)
    .with_processor(QcEphemerisCoverageProcessor::default())
    // optional: process [t0, t0 + 1h] only (context is synchronized anyway)
    .with_time_window(Some(t0), Some(t0 + Duration::from_hours(1.0)))
    // optional: context lookahead (default 3h)
    .with_lookahead(Duration::from_hours(3.0))
    .build()
    .unwrap();
```

| Builder | Description |
|---------|-------------|
| `with_products(products)` | Sources and static data of a `QcInputProducts` table |
| `with_source(source)` / `with_boxed_source(boxed)` | Any `QcSource` |
| `with_processor(processor)` | Any `QcProcessor` |
| `with_antex(indexing, rinex)` | Static antenna database |
| `with_time_window(start, end)` | Processing window (both optional) |
| `with_lookahead(duration)` | How far ahead context sources are pulled (default 3h) |
| `with_idle_sleep(duration)` | Sleep period of the blocking iterator, while sources are pending (default 1 ms) |

`build()` fails when there is no source (`QcEngineError::NoSource`), or when two signal sources
share the same identity (`QcEngineError::DuplicateSignalSource`): they must be merged or chained.

The engine can be consumed in three ways:

| API | Blocking | Use |
|-----|----------|-----|
| `for item in engine` | yes | `Iterator<Item = Result<QcOutputItem<O>, QcEngineError>>`: post processing |
| `engine.run(&mut sink)` | yes | streams everything into a `QcSink`, returns the `QcEngineStats` |
| `engine.poll_next()` | no | `QcPoll::{Ready, Pending, EndOfStream}`: event loops, async runtimes |

## Outputs

Results are `QcOutputItem<O>`: an epoch and a payload of type `O`.
Built-in processors produce `QcOutput`:

| `QcOutput` | Processor |
|------------|-----------|
| `Signal` | `QcSignalProcessor` |
| `EphemerisCoverage` | `QcEphemerisCoverageProcessor` |
| `Ephemeris` | `QcEphemerisProcessor` |
| `PVTSolution`, `NavigationReport` | `QcNavigationProcessor` (`navigation` feature) |

The output type is generic: applications define their own type (`O: From<QcOutput>`)
to stack their processors with the built-in ones, see [Processors](processors.md).

A `QcSink` consumes the stream. It is implemented for `Vec`, `std::sync::mpsc::Sender`,
and closures (`QcFnSink`); implement it for files, sockets, databases..:

```rust,no_run
use gnss_qc::prelude::*;

let engine = QcEngine::<QcOutput>::builder()
    .with_products(QcInputProducts::default())
    .with_processor(QcSignalProcessor)
    .build()
    .unwrap();

let mut count = 0;

let mut sink = QcFnSink(|item: QcOutputItem<QcOutput>| {
    count += 1;
    Ok::<(), ()>(())
});

let stats = engine.run(&mut sink).unwrap();
println!("{} results, {} epochs", count, stats.epochs);
```

With the `serde` feature, the output stream is serializable.

## Streaming files

Merging files in the products table holds all of them in memory. Chaining sources processes
them one after the other, and lazy sources only open a file when the engine needs it:
48h of observations are processed with a single day in memory.

```rust,no_run
use gnss_qc::{io::rinex::QcRinexObsSource, prelude::*};
use rinex::prelude::Rinex;

let nya1 = QcIndexing::Receiver("TRIMBLE NETR9 #5207K82137".to_string());

let daily = |path: &'static str| {
    let indexing = nya1.clone();
    QcLazySource::new(QcSourceDescriptor::new(nya1.clone()).with_signals(), move || {
        let rinex = Rinex::from_gzip_file(path)?;
        Ok(Box::new(QcRinexObsSource::new(indexing, rinex)?) as Box<dyn QcSource>)
    })
};

let observations = QcChainSource::new(
    QcSourceDescriptor::new(nya1.clone()).with_signals(),
    vec![
        Box::new(daily("data/CRNX/V3/NYA100NOR_S_20241270000_01D_30S_MO.crx.gz")),
        Box::new(daily("data/CRNX/V3/NYA100NOR_S_20241280000_01D_30S_MO.crx.gz")),
    ],
);

let engine = QcEngine::<QcOutput>::builder()
    .with_source(observations)
    .with_processor(QcSignalProcessor)
    .build()
    .unwrap();
```

## Real-time operation

No async runtime is required: a real-time source returns `QcPoll::Pending` when nothing is
available. A socket, a serial port or a decoder typically runs in its own thread and feeds
the engine through a channel:

```rust,no_run
use std::{sync::mpsc, thread};
use gnss_qc::prelude::*;

let (tx, rx) = mpsc::channel::<QcInputItem>();

thread::spawn(move || {
    // decode your stream (RTCM, UBX, ..) and send QcInputItems
    // tx.send(QcInputItem::signal(epoch, flag, observation)).unwrap();
    drop(tx); // ends the stream
});

let source = QcChannelSource::new(
    QcSourceDescriptor::new(QcIndexing::Custom("rover".to_string())).with_signals(),
    rx,
);

let mut engine = QcEngine::<QcOutput>::builder()
    .with_source(source)
    .with_processor(QcSignalProcessor)
    .build()
    .unwrap();

loop {
    match engine.poll_next().unwrap() {
        QcPoll::Ready(result) => { /* forward the result */ },
        QcPoll::Pending => { /* do something else, or wait */ },
        QcPoll::EndOfStream => break,
    }
}
```

`poll_next` is also the entry point to wrap the engine into an async stream, if your
application uses an async runtime.

## Memory and performance

- Sources are drained item by item: parsed records are released as the engine progresses.
- Context buffers hold a few data points per satellite: 48h of GPS + Galileo ephemerides
  (~1900 frames) never buffer more than ~220 of them, with no growth from one day to the next.
- Per epoch buffers are reused: no allocation per data point on the engine side.
- Measured (release build): 48h of a 30s multi-GNSS receiver (5760 epochs, 3.5 million
  signals) + 48h of ephemerides, ephemeris coverage analysis: 2.7 s, decompression and
  parsing included ([miso example](../examples/miso.rs)).

Next: [Processors →](processors.md)
