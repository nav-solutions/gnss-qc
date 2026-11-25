//! Real-time inputs: NTRIP clients (as many as needed), each streaming one mount point
//! of a caster into the MISO engine (RTCM 3: MSM observations, ephemerides, station coordinates).
//!
//! Each stream is described by `ROLE=URL`, ROLE being `rover`, `base` or `ephemerides`:
//!
//! ```bash
//! # one rover: signals and ephemerides of one station
//! cargo run --release --features ntrip --example ntrip -- \
//!     rover=ntrip://caster.centipede.fr:2101/MOUNT
//!
//! # RTK: a rover and a base (credentials in the URL), real-time P.V.T solutions
//! cargo run --release --features ntrip,navigation --example ntrip -- \
//!     rover=ntrip://user:password@caster:2101/ROVER \
//!     base=ntrip://user:password@caster:2101/BASE
//! ```
//!
//! Press Ctrl+C to stop.
use std::time::{Duration as StdDuration, Instant};

use gnss_qc::prelude::*;

#[cfg(feature = "navigation")]
#[path = "common/mod.rs"]
mod common;

/// Prints a summary of each epoch
struct EpochPrinter;

impl QcProcessor<QcOutput> for EpochPrinter {
    fn name(&self) -> &str {
        "epoch-printer"
    }

    fn process(&mut self, ctx: &QcEpochContext, _: &mut QcOutputQueue<QcOutput>) {
        for rx in ctx.receivers.iter() {
            let mut satellites = rx.signals.iter().map(|s| s.sv).collect::<Vec<_>>();
            satellites.sort();
            satellites.dedup();

            println!(
                "{} {} - {} signals, {} satellites, position {:?}, {} ephemerides",
                ctx.epoch,
                rx.receiver,
                rx.signals.len(),
                satellites.len(),
                rx.reference_position_ecef_m,
                ctx.ephemerides.len(),
            );
        }
    }
}

fn main() {
    let streams = std::env::args()
        .skip(1)
        .map(|arg| {
            let (role, url) = arg
                .split_once('=')
                .unwrap_or_else(|| panic!("expecting ROLE=URL, got \"{}\"", arg));

            let role = role.parse::<QcNtripRole>().unwrap();
            url.parse::<QcNtripConfig>().unwrap().with_role(role)
        })
        .collect::<Vec<_>>();

    if streams.is_empty() {
        eprintln!("usage: ntrip ROLE=ntrip://[user[:password]@]host[:port]/MOUNT ..");
        std::process::exit(1);
    }

    let mut builder = QcEngine::<QcOutput>::builder().with_processor(EpochPrinter);

    for stream in streams.iter() {
        println!("{} ({})", stream, stream.role);
        builder = builder.with_source(QcNtripSource::new(stream.clone()).unwrap());
    }

    #[cfg(feature = "navigation")]
    if streams.iter().any(|s| s.role == QcNtripRole::Rover) {
        let cfg = QcNavigationConfig::default()
            .with_method(Method::SPP)
            .with_ntrip_roles(&streams);

        builder =
            builder.with_processor(QcNavigationProcessor::new(common::almanac(), cfg).unwrap());
    }

    let mut engine = builder.build().unwrap();
    let mut last_report = Instant::now();

    // event loop: the engine never blocks
    loop {
        match engine.poll_next() {
            Ok(QcPoll::Ready(item)) => {
                #[cfg(feature = "navigation")]
                if let QcOutput::PVTSolution(solution) = &item.payload {
                    println!(
                        "{} {} - P.V.T {:?} ({:?})",
                        item.epoch,
                        solution.receiver,
                        solution.solution.pos_m,
                        solution.solution.solution_type
                    );
                }

                #[cfg(not(feature = "navigation"))]
                let _ = item;
            }
            Ok(QcPoll::Pending) => {
                if last_report.elapsed() > StdDuration::from_secs(10) {
                    println!("waiting for data.. ({:?})", engine.stats());
                    last_report = Instant::now();
                }
                std::thread::sleep(StdDuration::from_millis(20));
            }
            Ok(QcPoll::EndOfStream) => {
                println!("end of streams");
                break;
            }
            Err(e) => {
                eprintln!("{}", e);
                break;
            }
        }
    }
}
