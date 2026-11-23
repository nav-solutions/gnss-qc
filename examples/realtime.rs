//! Real-time operation: a thread decodes a stream (here: a file replayed with delays,
//! it could be a socket or a serial port) and feeds the engine through a channel.
//! The engine is polled without blocking, like in an event loop: no async runtime needed.
//!
//! ```bash
//! cargo run --release --example realtime
//! ```
use std::{sync::mpsc, thread, time::Duration as StdDuration};

use gnss_qc::{io::rinex::QcRinexObsSource, prelude::*};
use rinex::prelude::Rinex;

fn main() {
    let receiver = QcIndexing::Receiver("LEICA GR50 #1833574".to_string());
    let path = format!(
        "{}/data/OBS/V3/ACOR00ESP_R_20213550000_01D_30S_MO.rnx",
        env!("CARGO_MANIFEST_DIR")
    );

    let (tx, rx) = mpsc::channel();

    // producer thread: decodes and sends one data point at a time
    let indexing = receiver.clone();
    let producer = thread::spawn(move || {
        let rinex = Rinex::from_file(&path).unwrap();
        let mut decoder = QcRinexObsSource::new(indexing, rinex).unwrap();

        while let QcPoll::Ready(item) = decoder.poll().unwrap() {
            tx.send(item).unwrap();
            thread::sleep(StdDuration::from_micros(100));
        }
        // dropping the sender ends the stream
    });

    let stream = QcChannelSource::new(QcSourceDescriptor::new(receiver).with_signals(), rx);

    let mut engine = QcEngine::<QcOutput>::builder()
        .with_source(stream)
        .with_processor(QcSignalProcessor)
        .build()
        .unwrap();

    let (mut signals, mut pending) = (0, 0);

    loop {
        match engine.poll_next().unwrap() {
            QcPoll::Ready(item) => {
                if let QcOutput::Signal(signal) = item.payload {
                    signals += 1;
                    if signals % 1000 == 0 {
                        println!(
                            "{} - {} {:?}",
                            item.epoch, signal.signal.sv, signal.signal.observable
                        );
                    }
                }
            }
            QcPoll::Pending => {
                // nothing available yet: do something else, or wait
                pending += 1;
                thread::sleep(StdDuration::from_millis(1));
            }
            QcPoll::EndOfStream => break,
        }
    }

    producer.join().unwrap();

    println!(
        "{} signals, {} epochs, {} pending polls",
        signals,
        engine.stats().epochs,
        pending
    );
}
