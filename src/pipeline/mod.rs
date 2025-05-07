pub mod errors;
mod ports;
mod scheduler;
mod topology;
mod types;

use ports::QcElementPort;
use topology::Node;

// pub trait ScheduledElement {
//     /// True if this [QcPipelineElement] has input data ready to be consumed
//     fn has_input_data(&self) -> bool;

//     /// Process input data (consume)
//     fn process(&mut self);
// }

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

use crate::serializer::data::QcSerializedItem;
use crossbeam_channel::{Receiver, Sender};
use types::QcDataType;

pub struct QcPipelineSource {
    /// Input [QcDataType]
    input_dtype: QcDataType,

    /// Input port
    rx: Receiver<QcSerializedItem>,

    /// Output [QcDataType]
    output_dtype: QcDataType,
}

pub struct QcPipelineElement {
    /// Input [QcDataType]
    input_dtype: QcDataType,

    /// Output [QcDataType]
    output_dtype: QcDataType,
}

/// [QcPipeline]
pub struct QcPipeline {
    source: QcPipelineSource,
    elements: Vec<QcPipelineElement>,
}

impl QcPipeline {
    /// Deploy & execute this [QcPipeline]
    pub fn run(&mut self) {
        loop {}
    }
}

// #[cfg(test)]
// mod test {

//     use super::{QcDataType, topology::Topology, QcPipeline};

//     #[test]
//     fn pipeline_designer() {

//         let topology = Topology::new()
//             .add_source_node("src_1", QcDataType::QcEphemerisData)
//             .add_node(node)
//     }
// }
