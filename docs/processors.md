Processors
==========

[← MISO engine](miso.md) · [README](../README.md) · [Navigation →](navigation/README.md)

Processors are the algorithms run by the engine. At each epoch, every processor receives the
synchronized `QcEpochContext` and produces results into the output queue.

## Built-in processors

| Processor | Output | Description |
|-----------|--------|-------------|
| `QcSignalProcessor` | `QcOutput::Signal` | Streams every signal observation out, as is, for all receivers |
| `QcEphemerisCoverageProcessor` | `QcOutput::EphemerisCoverage` | Verifies that every satellite in sight is covered by a valid radio ephemeris. Emitted on change only: coverage gained, lost, or switch to another ephemeris |
| `QcEphemerisProcessor` | `QcOutput::Ephemeris` | Streams every radio ephemeris out, once, as soon as it reaches the engine |
| `QcNavigationProcessor` | `QcOutput::PVTSolution`, `QcOutput::NavigationReport` | P.V.T solutions (`navigation` feature), see [Navigation](navigation/README.md) |

## The epoch context

| `QcEpochContext` field | Content |
|------------------------|---------|
| `epoch` | Epoch being processed |
| `receivers` | One `QcEpochSignals` per receiver that sampled this epoch: identity, epoch flag, receiver clock, reference position (if known), signal observations |
| `ephemerides` | `QcEphemerisStore`, synchronized to this epoch |
| `precise_states` | `QcPreciseStore`, synchronized to this epoch |
| `statics` | Session wide data (ANTEX) |

## Writing a processor

A processor implements `QcProcessor<O>`, `O` being the output type of the engine:

```rust,ignore
pub trait QcProcessor<O = QcOutput> {
    fn name(&self) -> &str;
    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>);
    fn finalize(&mut self, _output: &mut QcOutputQueue<O>) {}
}
```

- `process` is called once per epoch, in chronological order;
- `finalize` is called once, when all signal sources have ended (summaries, reports).

Applications define their own output type to stack their processors with the built-in ones.
It only needs to implement `From<QcOutput>`:

```rust,no_run
use gnss_qc::prelude::*;

/// Application output: built-in results + application results
#[derive(Debug)]
enum AppOutput {
    Builtin(QcOutput),
    SatellitesInSight { receiver: QcIndexing, count: usize },
    Summary { epochs: usize },
}

impl From<QcOutput> for AppOutput {
    fn from(output: QcOutput) -> Self {
        Self::Builtin(output)
    }
}

/// Counts the satellites in sight of each receiver
#[derive(Default)]
struct SatellitesInSight {
    epochs: usize,
    svs: Vec<SV>, // reused buffer
}

impl QcProcessor<AppOutput> for SatellitesInSight {
    fn name(&self) -> &str {
        "satellites-in-sight"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<AppOutput>) {
        self.epochs += 1;

        for rx in ctx.receivers {
            self.svs.clear();
            self.svs.extend(rx.signals.iter().map(|signal| signal.sv));
            self.svs.sort();
            self.svs.dedup();

            output.push(
                ctx.epoch,
                AppOutput::SatellitesInSight {
                    receiver: rx.receiver.clone(),
                    count: self.svs.len(),
                },
            );
        }
    }

    fn finalize(&mut self, output: &mut QcOutputQueue<AppOutput>) {
        output.push(Epoch::default(), AppOutput::Summary { epochs: self.epochs });
    }
}

// built-in and custom processors, in the same engine, same output stream
let engine = QcEngine::<AppOutput>::builder()
    .with_products(QcInputProducts::default())
    .with_processor(QcEphemerisCoverageProcessor::default())
    .with_processor(SatellitesInSight::default())
    .build()
    .unwrap();

for item in engine {
    match item.unwrap().payload {
        AppOutput::SatellitesInSight { receiver, count } => println!("{}: {}", receiver, count),
        AppOutput::Summary { epochs } => println!("{} epochs", epochs),
        AppOutput::Builtin(QcOutput::EphemerisCoverage(coverage)) => println!("{:?}", coverage),
        AppOutput::Builtin(_) => {},
    }
}
```

Guidelines:

- keep processing per epoch: the engine streams, processors should not accumulate the
  whole session (emit results as they come, summaries in `finalize`);
- reuse buffers between epochs;
- processors run in insertion order, in the engine thread.

Next: [Navigation →](navigation/README.md)
