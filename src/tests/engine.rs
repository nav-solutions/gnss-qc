use std::{str::FromStr, sync::mpsc, thread, time::Duration as StdDuration};

use crate::prelude::*;
use crate::tests::toolkit::{ephemeris_item, epoch, observations, pseudo_range_item};

fn rx(name: &str) -> QcIndexing {
    QcIndexing::Receiver(name.to_string())
}

fn agency(name: &str) -> QcIndexing {
    QcIndexing::Agency(name.to_string())
}

fn obs_source(
    name: &str,
    items: Vec<QcInputItem>,
) -> QcIterSource<std::vec::IntoIter<QcInputItem>> {
    QcIterSource::new(
        QcSourceDescriptor::new(rx(name)).with_signals(),
        items.into_iter(),
    )
}

fn nav_source(
    name: &str,
    items: Vec<QcInputItem>,
) -> QcIterSource<std::vec::IntoIter<QcInputItem>> {
    QcIterSource::new(
        QcSourceDescriptor::new(agency(name)).with_ephemerides(),
        items.into_iter(),
    )
}

/// Application specific output type, stacking built-in and custom results.
#[derive(Debug, Clone, PartialEq)]
enum AppOutput {
    Builtin(QcOutput),
    Receivers(usize),
}

impl From<QcOutput> for AppOutput {
    fn from(output: QcOutput) -> Self {
        Self::Builtin(output)
    }
}

/// Custom processor: counts receivers in each epoch, and checks synchronization invariants.
#[derive(Default)]
struct ReceiversCounter {
    last: Option<Epoch>,
}

impl QcProcessor<AppOutput> for ReceiversCounter {
    fn name(&self) -> &str {
        "receivers-counter"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<AppOutput>) {
        // chronological processing
        if let Some(last) = self.last {
            assert!(ctx.epoch > last, "epochs must be strictly increasing");
        }
        self.last = Some(ctx.epoch);

        // memory stays bounded: a few ephemerides per satellite at most
        assert!(ctx.ephemerides.len() <= 6, "ephemerides not pruned");

        output.push(ctx.epoch, AppOutput::Receivers(ctx.receivers.len()));
    }
}

#[test]
fn builder_errors() {
    let t0 = epoch("2020-06-25T00:00:00 GPST");

    let engine = QcEngine::<QcOutput>::builder().build();
    assert!(matches!(engine, Err(QcEngineError::NoSource)));

    // context only session is valid
    let engine = QcEngine::<QcOutput>::builder()
        .with_source(nav_source("X", vec![ephemeris_item("G01", t0)]))
        .build();
    assert!(engine.is_ok());

    let engine = QcEngine::<QcOutput>::builder()
        .with_source(obs_source("A", vec![]))
        .with_source(obs_source("A", vec![]))
        .build();
    assert!(matches!(
        engine,
        Err(QcEngineError::DuplicateSignalSource(_))
    ));
}

#[test]
fn misaligned_receivers_and_ephemeris_coverage() {
    let t0 = epoch("2020-06-25T00:00:00 GPST");
    let h = |hours: f64| Duration::from_hours(hours);
    let m = |minutes: f64| Duration::from_seconds(minutes * 60.0);

    // receiver A: G01 + G02 every 30s, [t0, t0+3h[
    let a = observations(
        &["G01", "G02"],
        t0,
        t0 + h(3.0),
        Duration::from_seconds(30.0),
    );

    // receiver B: G01 every 60s, [t0+15', t0+4h[
    let b = observations(&["G01"], t0 + m(15.0), t0 + h(4.0), m(1.0));

    // agency X: G01 every 2h, G02 only once (coverage loss after t0+2h)
    let nav = vec![
        ephemeris_item("G01", t0),
        ephemeris_item("G02", t0),
        ephemeris_item("G01", t0 + h(2.0)),
        ephemeris_item("G01", t0 + h(4.0)),
    ];

    let engine = QcEngine::<AppOutput>::builder()
        .with_source(obs_source("A", a))
        .with_source(nav_source("X", nav))
        .with_source(obs_source("B", b))
        .with_processor(QcEphemerisCoverageProcessor::default())
        .with_processor(ReceiversCounter::default())
        .build()
        .unwrap();

    let mut results = Vec::<QcOutputItem<AppOutput>>::new();
    let stats = engine.run(&mut results).unwrap();

    // union of epochs: 360 (A) + 60 (B alone, last hour)
    assert_eq!(stats.epochs, 420);
    assert_eq!(stats.late_items, 0);
    assert_eq!(stats.items, 360 * 2 + 225 + 4);

    // synchronization: both receivers are presented together on common epochs
    let receivers_at = |t: Epoch| {
        results
            .iter()
            .find_map(|item| match item.payload {
                AppOutput::Receivers(n) if item.epoch == t => Some(n),
                _ => None,
            })
            .unwrap()
    };

    assert_eq!(receivers_at(t0), 1);
    assert_eq!(receivers_at(t0 + Duration::from_seconds(30.0)), 1);
    assert_eq!(receivers_at(t0 + m(15.0)), 2);
    assert_eq!(receivers_at(t0 + m(16.0)), 2);
    assert_eq!(receivers_at(t0 + h(3.5)), 1);

    // coverage transitions
    let coverage = results
        .iter()
        .filter_map(|item| match &item.payload {
            AppOutput::Builtin(QcOutput::EphemerisCoverage(cov)) => Some((
                item.epoch,
                cov.receiver.clone(),
                cov.sv.to_string(),
                cov.toc,
            )),
            _ => None,
        })
        .collect::<Vec<_>>();

    let expected = vec![
        (t0, rx("A"), "G01".to_string(), Some(t0)),
        (t0, rx("A"), "G02".to_string(), Some(t0)),
        (t0 + m(15.0), rx("B"), "G01".to_string(), Some(t0)),
        (t0 + m(60.5), rx("A"), "G01".to_string(), Some(t0 + h(2.0))),
        (t0 + m(61.0), rx("B"), "G01".to_string(), Some(t0 + h(2.0))),
        (t0 + h(2.0), rx("A"), "G02".to_string(), None),
        (t0 + m(181.0), rx("B"), "G01".to_string(), Some(t0 + h(4.0))),
    ];

    assert_eq!(coverage, expected);
}

#[test]
fn late_observations_are_dropped() {
    let t0 = epoch("2020-06-25T00:00:00 GPST");
    let dt = Duration::from_seconds(30.0);

    let items = vec![
        pseudo_range_item("G01", t0, 1.0),
        pseudo_range_item("G01", t0 + dt, 2.0),
        pseudo_range_item("G01", t0, 3.0), // late: t0 already processed
        pseudo_range_item("G01", t0 + dt * 2, 4.0),
    ];

    let engine = QcEngine::<QcOutput>::builder()
        .with_source(obs_source("A", items))
        .with_processor(QcSignalProcessor)
        .build()
        .unwrap();

    let mut results = Vec::new();
    let stats = engine.run(&mut results).unwrap();

    assert_eq!(stats.epochs, 3);
    assert_eq!(stats.late_items, 1);

    let values = results
        .iter()
        .map(|item| match &item.payload {
            QcOutput::Signal(sig) => sig.signal.value,
            _ => panic!("unexpected output"),
        })
        .collect::<Vec<_>>();

    assert_eq!(values, vec![1.0, 2.0, 4.0]);
}

#[test]
fn realtime_channel_source() {
    let t0 = epoch("2020-06-25T00:00:00 GPST");
    let dt = Duration::from_seconds(1.0);

    let (tx, rx_chan) = mpsc::channel();

    // "socket" thread: one epoch every few ms, ephemeris arrives along the stream
    let producer = thread::spawn(move || {
        tx.send(ephemeris_item("G01", t0)).unwrap();
        for i in 0..10 {
            let t = t0 + dt * i;
            tx.send(pseudo_range_item("G01", t, 20.0E6)).unwrap();
            tx.send(pseudo_range_item("G02", t, 21.0E6)).unwrap();
            thread::sleep(StdDuration::from_millis(5));
        }
        // dropping tx ends the stream
    });

    let source = QcChannelSource::new(
        QcSourceDescriptor::new(QcIndexing::Custom("rtcm".to_string()))
            .with_signals()
            .with_ephemerides(),
        rx_chan,
    );

    let mut engine = QcEngine::<QcOutput>::builder()
        .with_source(source)
        .with_processor(QcEphemerisCoverageProcessor::default())
        .build()
        .unwrap();

    let mut pending = 0;
    let mut results = Vec::new();

    loop {
        match engine.poll_next().unwrap() {
            QcPoll::Ready(item) => results.push(item),
            QcPoll::Pending => {
                pending += 1;
                thread::sleep(StdDuration::from_millis(1));
            }
            QcPoll::EndOfStream => break,
        }
    }

    producer.join().unwrap();

    assert!(pending > 0, "real-time source should have been pending");
    assert_eq!(engine.stats().epochs, 10);

    // G01 covered (ephemeris from the same stream), G02 never covered
    let g01 = SV::from_str("G01").unwrap();
    let g02 = SV::from_str("G02").unwrap();

    let coverage = results
        .iter()
        .filter_map(|item| match &item.payload {
            QcOutput::EphemerisCoverage(cov) => Some((item.epoch, cov.sv, cov.toc)),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(coverage, vec![(t0, g01, Some(t0)), (t0, g02, None)]);
}
