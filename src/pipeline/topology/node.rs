use crate::{
    pipeline::{
        errors::TopologyError, types::QcDataType, QcPipeline, QcPipelineElement, QcPipelineSource,
    },
    serializer::data::QcSerializedItem,
};

use crossbeam_channel::Receiver;

/// [Node] describes an element of the [Topology] that are not wired yet.
#[derive(Clone)]
pub struct Node {
    /// Readable name (unique) for this [Node]
    pub name: String,

    /// Name of this [Node]'s parent.
    /// Sinks do not have parents.
    pub parent_name: Option<String>,

    /// Input [QcDataType]
    pub input_type: QcDataType,

    /// Output [QcDataType]
    pub output_type: QcDataType,
}

impl Node {
    /// Define a new [Topology] [Node]
    pub fn new(name: &str, input_type: QcDataType, output_type: QcDataType) -> Self {
        Self {
            name: name.to_string(),
            input_type,
            output_type,
            parent_name: None,
        }
    }

    /// Define that this [Node] has a parent
    pub fn with_parent(&self, name: &str) -> Self {
        let mut s = self.clone();
        s.parent_name = Some(name.to_string());
        s
    }
}
