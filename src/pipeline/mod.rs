use errors::TopologyError;
use topology::Topology;

use crate::serializer::data::{QcSerializedData, QcSerializedItem};

pub mod errors;
mod ports;
mod scheduler;
mod topology;
mod types;

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


use crossbeam_channel::Receiver;

pub struct QcPipeline {
    pub topology: Topology, 
    pub serializer_rx: Receiver<QcSerializedItem>,
}

impl QcPipeline {
    /// Deploy & execute this [QcPipeline]
    pub fn run(&mut self) -> Result<(), TopologyError> {
        
        let source = self.topology.get_source_node()
            .ok_or(TopologyError::UndefinedSourceEntryPoint)?;

        loop {
            // block on data source
            match self.serializer_rx.recv() {
                Ok(value) => {

                    source.push(value);
                    // do some work
                    // for node in self.topology.nodes.iter() {
                    //     if node.can_process() {
                    //         node.process();
                    //     }
                    // }

                },
                Err(_) => {
                    // a message could not be received because the channel is disconnected.
                    break;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {



}
//     use super::QcPipeline;

//     #[test]
//     fn simple_pipeline_test() {
//         let topology = Topology::new()
//             .entrypoint("eph-source", QcDataType::QcEphemerisData)
//             .node(
//                 "eph-processor#1",
//                 "eph-source",
//                 QcDataType::QcEphemerisData,
//                 QcDataType::QcEphemerisData,
//             )
//             .unwrap();

//         let pipeline = QcPipeline {
//             topology,
//         };

//         let (fake_tx, fake_rx) = crossbeam_channel::bounded(128);

//         let pipeline = topology.wire(fake_rx)
//             .unwrap();

//         loop {
//             for node in pipeline.elements.iter() {

//             }
//         }
//     }
// }
