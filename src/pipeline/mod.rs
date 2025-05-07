pub mod errors;
mod ports;
mod topology;
mod types;

use ports::QcElementPort;

// pub trait ScheduledElement {
//     /// True if this [QcPipelineElement] has input data ready to be consumed
//     fn has_input_data(&self) -> bool;

//     /// Process input data (consume)
//     fn process(&mut self);
// }

/// [QcPipelineElement] describes an element of the topology
/// that is wired to a source and possibly a child.
pub struct QcPipelineElement {
    rx_port: QcElementPort,
}

// pub struct QcPipeline<I: Send> {
//     elements: Vec<QcPipelineElement<I>>,
// }

// // let (serializer_tx, entrypoints_rx) = crossbeam_channel::unbounded();
// // let (obs_tx, obs_rx) = crossbeam_channel::unbounded();
// // let (ephemeris_tx, ephemeris_rx) = crossbeam_channel::unbounded();

// // let mut ephemeris_streamer =
// //     QcEphemerisStreamer::new("eph-streamer", entrypoints_rx.clone(), ephemeris_tx);

// // let mut obs_streamer =
// //     QcObservationsStreamer::new("obs-streamer", entrypoints_rx.clone(), obs_tx);
