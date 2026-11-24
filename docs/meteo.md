Meteo observations
==================

[← Precise clocks](clocks.md) · [README](../README.md) · [Processors →](processors.md)

Meteo RINEX files describe the observations of meteo sensors, usually colocated with a
GNSS station: pressure, temperature, humidity, zenith delays, wind, rain..
gnss-qc accepts them as inputs: they are synchronized like every other context product,
available to every processor, and streamed out by the `QcMeteoProcessor`.

## Loading

Meteo RINEX are loaded like any other file (RINEX V2, V3), indexed by agency or
file name, see [indexing](context.md#indexing):

```rust,no_run
use gnss_qc::prelude::*;

let mut products = QcInputProducts::default();

products
    .load_gzip_file(
        &QcInputConfig::default(),
        "data/MET/V3/POTS00DEU_R_20232540000_01D_05M_MM.rnx.gz",
    )
    .unwrap();

// agency:GFZ
for (source, _) in products.meteo_rinex_iter() {
    println!("{}", source);
}
```

Each Meteo RINEX becomes a `QcRinexMeteoSource`, which delivers one `QcPayload::Meteo`
per observation (`QcMeteoSample`: observable and value), in chronological order.
Any source declaring `QcSourceDescriptor::with_meteo()` may deliver meteo observations
(a sensor connected to a serial port for example).

| `Observable` | Unit |
|--------------|------|
| `Pressure` | hPa |
| `Temperature` | °C |
| `HumidityRate` | % |
| `ZenithDryDelay`, `ZenithWetDelay`, `ZenithTotalDelay` | mm |
| `WindDirection` | ° |
| `WindSpeed` | m/s |
| `RainIncrement` | 1/10 mm |
| `HailIndicator` | - |

## Synchronization

The engine buffers the observations in the `QcMeteoStore`, per source and per observable,
around the epoch being processed (3 hours retention). It is available to every processor
(`QcEpochContext::meteo`):

| `QcMeteoStore` | Description |
|----------------|-------------|
| `latest(source, observable, t)` | Latest observation at or before `t` |
| `interpolate_from(source, observable, t)` | Linear interpolation at `t` (no extrapolation, no interpolation across data gaps) |
| `interpolate(observable, t)` | Same, from the first source that can |
| `samples(source, observable)` | Buffered observations, time ordered |
| `observables_of(source)`, `sources()` | Content of the store |

```rust,no_run
use gnss_qc::prelude::*;
use rinex::prelude::Observable;

struct MyProcessor;

impl QcProcessor<QcOutput> for MyProcessor {
    fn name(&self) -> &str {
        "my-processor"
    }

    fn process(&mut self, ctx: &QcEpochContext, _: &mut QcOutputQueue<QcOutput>) {
        if let Some((station, pressure)) = ctx.meteo.interpolate(&Observable::Pressure, ctx.epoch) {
            println!("{} {} {:.1} hPa", ctx.epoch, station, pressure);
        }
    }
}
```

## Meteo stream

`QcMeteoProcessor` streams the observations out (`QcOutput::Meteo`), once, in chronological
order, time tagged with the observation epoch. `QcMeteoConfig::observables` restricts the
stream to some observables.

```rust,no_run
use gnss_qc::prelude::*;
use rinex::prelude::Observable;

let mut products = QcInputProducts::default();

products
    .load_gzip_file(
        &QcInputConfig::default(),
        "data/MET/V3/POTS00DEU_R_20232540000_01D_05M_MM.rnx.gz",
    )
    .unwrap();

let engine = QcEngine::<QcOutput>::builder()
    .with_products(products)
    .with_processor(QcMeteoProcessor::new(
        QcMeteoConfig::default().with_observables(&[Observable::Pressure, Observable::Temperature]),
    ))
    .build()
    .unwrap();

for item in engine {
    let item = item.unwrap();
    if let QcOutput::Meteo(sample) = item.payload {
        println!("{} {} {} {}", item.epoch, sample.source, sample.observable, sample.value);
    }
}
```

```bash
cargo run --release --example meteo > meteo.csv
```

Verified by the test suite (`src/tests/clock`): every observation of the POTS00DEU
(2023-09-11, 5', pressure, temperature, humidity) and ABVI (2015-01-01, RINEX V2) files is
streamed once with its value, both files in the same session.

## Limitations

- The navigation does not use the meteo observations yet: the troposphere delay is
  modeled (Niell). A troposphere model driven by the observed pressure, temperature and
  humidity (or the observed zenith delays) is the natural next step.
- The position of the sensors (header) is not used.

Next: [Processors →](processors.md)
