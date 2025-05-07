use crate::pipeline::{errors::TopologyError, types::QcDataType};

/// [Node] describes an element of the [Topology] that are not wired yet.
#[derive(Clone)]
struct Node {
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

/// [Topology] defines a Pipeline topology, but it is not wired yet.
#[derive(Clone)]
pub struct Topology {
    /// List of [Node]s
    nodes: Vec<Node>,
}

impl Topology {
    /// Define a new [Topology]
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Add a source [Node] to this [Topology]
    pub fn add_source_node(&self, name: &str, dtype: QcDataType) -> Self {
        let mut s = self.clone();

        s.nodes.push(Node {
            name: name.to_string(),
            parent_name: None,
            input_type: QcDataType::QcWrappedData,
            output_type: dtype,
        });

        s
    }

    pub fn add_node(&self, node: Node) -> Result<Self, TopologyError> {
        let parent = node
            .parent_name
            .as_ref()
            .ok_or(TopologyError::UndefinedParentName)?;

        if !self.node_exists(parent) {
            return Err(TopologyError::ParentDoesNotExist(parent.to_string()));
        }

        if self.node_exists(&node.name) {
            return Err(TopologyError::NodeAlreadyExists(node.name.to_string()));
        }

        let mut s = self.clone();
        s.nodes.push(node);
        Ok(s)
    }

    /// Verify this topology is correct, using basic verifications
    /// - single source (entry point)
    fn is_valid(&self) -> Result<(), TopologyError> {
        let mut nb_sources = 0;

        for node in self.nodes.iter() {
            if node.parent_name.is_none() {
                nb_sources += 1;
            }
        }

        if nb_sources == 0 {
            return Err(TopologyError::UndefinedSourceEntryPoint);
        }

        if nb_sources > 1 {
            return Err(TopologyError::SourceIsNotUnique);
        }

        Ok(())
    }

    /// Obtain reference to given node (by name)
    fn get_node(&self, name: &str) -> Option<&Node> {
        self.nodes.iter().find(|node| node.name == name)
    }

    /// Obtain reference to the source node (route entry point)
    fn get_source_node(&self) -> Option<&Node> {
        self.nodes.iter().find(|node| node.parent_name.is_none())
    }

    /// True if this node exists
    fn node_exists(&self, name: &str) -> bool {
        for node in self.nodes.iter() {
            if node.name == name {
                return true;
            }
        }

        false
    }

    /// True if at least one node is this node's child
    fn has_child(&self, name: &str) -> bool {
        for node in self.nodes.iter() {
            if let Some(parent) = &node.parent_name {
                if parent == name {
                    return true;
                }
            }
        }

        false
    }

    /// Returns total number of nodes, including sink
    pub fn total_nodes(&self) -> usize {
        self.nodes.iter().count()
    }

    /// Connects every [Node]s described in this [Topology], creating a [QcPipeline].
    pub fn wire(&mut self) -> Result<(), TopologyError> {
        // let mut elements = Vec::with_capacity(8);

        // verify topology sanity
        self.is_valid()?;

        // // start with sink element
        // let sink_node = self.get_sink_node().ok_or({
        //     error!("invalid topology: sink node is not defined");
        //     Err(TopologyError::UndefinedSink)
        // })?;

        // for node in self.nodes.iter() {
        //     if let Some(parent) = &node.parent_name {

        //         // connect all nodes to their parent
        //         let parent_node = self.get_node(&parent).ok_or({
        //             error!(
        //                 "invalid topology: referenced parent \"{}\" does not exist",
        //                 parent
        //             );
        //             Err(TopologyError::ParentDoesNotExist(parent.to_string()))
        //         })?;

        //         let (tx, rx) = crossbeam_channel::unbounded::<QcSerializedItem>();

        //         let element = QcPipelineElement {
        //             rx_port: rx,
        //         };
        //     }
        // }

        // Ok(QcPipeline { elements })
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use crate::pipeline::{topology::*, types::QcDataType};

    #[test]
    fn topology_designer_1() {
        let topology = Topology::new();

        // |--------|
        // |source_1|
        // |--------|
        //      ^----------node_1----node_1_1
        //     ^-----------node_2----node_2_1----node_2_1_1

        assert_eq!(topology.total_nodes(), 0);

        let topology = topology.add_source_node("src_1", QcDataType::QcEphemerisData);

        assert_eq!(topology.total_nodes(), 1);

        let node_1 = Node::new(
            "node_1",
            QcDataType::QcEphemerisData,
            QcDataType::QcEphemerisData,
        )
        .with_parent("src_1");

        let topology = topology.add_node(node_1).unwrap();

        assert_eq!(topology.total_nodes(), 2);

        let node_2 = Node::new(
            "node_2",
            QcDataType::QcEphemerisData,
            QcDataType::QcEphemerisData,
        )
        .with_parent("src_1");

        let topology = topology.add_node(node_2).unwrap();

        assert_eq!(topology.total_nodes(), 3);

        let node_1_1 = Node::new(
            "node_1_1",
            QcDataType::QcEphemerisData,
            QcDataType::QcEphemerisData,
        )
        .with_parent("node_1");

        let topology = topology.add_node(node_1_1).unwrap();

        assert_eq!(topology.total_nodes(), 4);

        let node_2_1 = Node::new(
            "node_2_1",
            QcDataType::QcEphemerisData,
            QcDataType::QcEphemerisData,
        )
        .with_parent("node_2");

        let topology = topology.add_node(node_2_1).unwrap();

        assert_eq!(topology.total_nodes(), 5);

        let node_2_1_1 = Node::new(
            "node_2_1_1",
            QcDataType::QcEphemerisData,
            QcDataType::QcEphemerisData,
        )
        .with_parent("node_2_1");

        let topology = topology.add_node(node_2_1_1).unwrap();

        assert_eq!(topology.total_nodes(), 6);
    }
}
