//! The same file, replayed through a real-time "socket" (channel fed by a thread),
//! gives exactly the same results as the file itself: the core does not depend
//! on the nature of its inputs.
use std::{sync::mpsc, thread, time::Duration as StdDuration};

use super::*;

const OBS: &str = "OBS/V3/ACOR00ESP_R_20213550000_01D_30S_MO.rnx";

fn acor() -> QcIndexing {
    QcIndexing::Receiver("LEICA GR50 #1833574".to_string())
}

/// Processes the file directly
fn from_file() -> RunResults {
    let source = QcRinexObsSource::new(acor(), load_rinex(OBS)).unwrap();

    let engine = QcEngine::<TestOutput>::builder()
        .with_source(source)
        .with_processor(QcSignalProcessor)
        .with_processor(EpochRecorder::default())
        .build()
        .unwrap();

    run(engine)
}

#[test]
fn file_replayed_as_realtime_stream() {
    let reference = from_file();

    // "socket" thread: the file is decoded item by item and sent with delays,
    // the engine sees an irregular, real-time, stream.
    let (tx, rx) = mpsc::channel();

    let producer = thread::spawn(move || {
        let mut file = QcRinexObsSource::new(acor(), load_rinex(OBS)).unwrap();
        let mut sent = 0;
        while let QcPoll::Ready(item) = file.poll().unwrap() {
            tx.send(item).unwrap();
            sent += 1;
            if sent % 100 == 0 {
                thread::sleep(StdDuration::from_millis(2));
            }
        }
        sent
    });

    let socket = QcChannelSource::new(QcSourceDescriptor::new(acor()).with_signals(), rx);

    let mut engine = QcEngine::<TestOutput>::builder()
        .with_source(socket)
        .with_processor(QcSignalProcessor)
        .with_processor(EpochRecorder::default())
        .build()
        .unwrap();

    // non blocking consumption, as an event loop would do
    let mut pending = 0;
    let mut epochs = Vec::new();
    let mut signals = 0;

    loop {
        match engine.poll_next().unwrap() {
            QcPoll::Ready(item) => match item.payload {
                TestOutput::Epoch(summary) => epochs.push(summary),
                TestOutput::Builtin(QcOutput::Signal(_)) => signals += 1,
                _ => {}
            },
            QcPoll::Pending => {
                pending += 1;
                thread::sleep(StdDuration::from_micros(200));
            }
            QcPoll::EndOfStream => break,
        }
    }

    let sent = producer.join().unwrap();

    assert!(pending > 0, "the stream should have been pending");
    assert_eq!(sent, 9036);
    assert_eq!(engine.stats().items, sent);
    assert_eq!(signals, reference.signals);
    assert_eq!(
        epochs, reference.epochs,
        "real-time and file results differ"
    );
}
