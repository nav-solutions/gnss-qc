use crate::{
    pipeline::{
        errors::TopologyError, topology::node::Node, types::QcDataType, QcPipeline,
        QcPipelineElement, QcPipelineSource,
    },
    serializer::data::QcSerializedItem,
};

use crossbeam_channel::Receiver;

/// [Topology] defines a Pipeline topology, but it is not wired yet.
#[derive(Clone, Default)]
pub struct Topology {
    /// List of [Node]s
    nodes: Vec<Node>,
}

impl Topology {
    /// Define a new [Topology]
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Define [Topology] entrypoint.
    pub fn entrypoint(&self, name: &str, dtype: QcDataType) -> Self {
        let mut s = self.clone();

        s.nodes.push(Node {
            name: name.to_string(),
            parent_name: None,
            input_type: QcDataType::QcWrappedData,
            output_type: dtype,
        });

        s
    }

    pub fn node(
        &self,
        name: &str,
        parent: &str,
        input_dtype: QcDataType,
        output_dtype: QcDataType,
    ) -> Result<Self, TopologyError> {
        if !self.node_exists(parent) {
            return Err(TopologyError::ParentDoesNotExist(parent.to_string()));
        }

        if self.node_exists(name) {
            return Err(TopologyError::NodeAlreadyExists(name.to_string()));
        }

        let mut s = self.clone();

        let node = Node::new(name, input_dtype, output_dtype).with_parent(parent);

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
    pub fn wire(
        &self,
        serializer_rx: Receiver<QcSerializedItem>,
    ) -> Result<QcPipeline, TopologyError> {
        // let mut elements = Vec::with_capacity(8);

        // verify topology sanity
        self.is_valid()?;

        let mut elements = Vec::<QcPipelineElement>::new();
        let mut wired_source = Option::<QcPipelineSource>::None;

        for node in self.nodes.iter() {
            let input_dtype = node.input_type;
            let output_dtype = node.output_type;

            if node.parent_name.is_none() {
                wired_source = Some(QcPipelineSource {
                    rx: serializer_rx.clone(),
                    input_dtype,
                    output_dtype,
                });
            } else {
                elements.push(QcPipelineElement {
                    input_dtype,
                    output_dtype,
                });
            }
        }

        let source = wired_source.ok_or(TopologyError::UndefinedSourceEntryPoint)?;

        Ok(QcPipeline { source, elements })
    }
}

#[cfg(test)]
mod test {

    use crate::pipeline::{topology::Topology, types::QcDataType};

    #[test]
    fn default_topology() {
        assert_eq!(
            Topology::new().total_nodes(),
            0,
            "default topology is not empty!"
        );
        assert_eq!(
            Topology::default().total_nodes(),
            0,
            "default topology is not empty!"
        )
    }

    #[test]
    fn source_only_topology() {
        let topology = Topology::new().entrypoint("src_1", QcDataType::QcEphemerisData);

        assert!(topology.node_exists("src_1"));
        assert_eq!(topology.total_nodes(), 1);

        let (_, fake_channel) = crossbeam_channel::bounded(128);

        let _ = topology.wire(fake_channel).unwrap_or_else(|e| {
            panic!("Failed to create QcPipeline: {}", e);
        });
    }

    #[test]
    fn one_level_topology() {
        let topology = Topology::new()
            .entrypoint("eph-source", QcDataType::QcEphemerisData)
            .node(
                "eph-processor#1",
                "eph-source",
                QcDataType::QcEphemerisData,
                QcDataType::QcEphemerisData,
            )
            .unwrap();

        assert!(topology.node_exists("eph-source"));
        assert!(topology.node_exists("eph-processor#1"));

        assert_eq!(topology.total_nodes(), 2);

        let (_, fake_channel) = crossbeam_channel::bounded(128);

        let _ = topology.wire(fake_channel).unwrap_or_else(|e| {
            panic!("Failed to create QcPipeline: {}", e);
        });
    }

    #[test]
    fn two_levels_topology() {
        let topology = Topology::new()
            .entrypoint("eph-source", QcDataType::QcEphemerisData)
            .node(
                "eph-processor#1",
                "eph-source",
                QcDataType::QcEphemerisData,
                QcDataType::QcEphemerisData,
            )
            .unwrap()
            .node(
                "eph-processor#2",
                "eph-processor#1",
                QcDataType::QcEphemerisData,
                QcDataType::QcEphemerisData,
            )
            .unwrap();

        assert!(topology.node_exists("eph-source"));
        assert!(topology.node_exists("eph-processor#1"));
        assert!(topology.node_exists("eph-processor#2"));

        assert_eq!(topology.total_nodes(), 3);

        let (_, fake_channel) = crossbeam_channel::bounded(128);

        let _ = topology.wire(fake_channel).unwrap_or_else(|e| {
            panic!("Failed to create QcPipeline: {}", e);
        });
    }

    #[test]
    fn two_branches_topology() {
        let topology = Topology::new()
            .entrypoint("eph-source", QcDataType::QcEphemerisData)
            .node(
                "eph-processor#1",
                "eph-source",
                QcDataType::QcEphemerisData,
                QcDataType::QcEphemerisData,
            )
            .unwrap()
            .node(
                "eph-processor#2",
                "eph-source",
                QcDataType::QcEphemerisData,
                QcDataType::QcEphemerisData,
            )
            .unwrap();

        assert!(topology.node_exists("eph-source"));
        assert!(topology.node_exists("eph-processor#1"));
        assert!(topology.node_exists("eph-processor#2"));

        assert_eq!(topology.total_nodes(), 3);

        let (_, fake_channel) = crossbeam_channel::bounded(128);

        let _ = topology.wire(fake_channel).unwrap_or_else(|e| {
            panic!("Failed to create QcPipeline: {}", e);
        });
    }

    #[test]
    fn complex_topology() {
        let topology = Topology::new()
            .entrypoint("eph-source", QcDataType::QcEphemerisData)
            .node(
                "eph-processor#1",
                "eph-source",
                QcDataType::QcEphemerisData,
                QcDataType::QcEphemerisData,
            )
            .unwrap()
            .node(
                "eph-processor#2",
                "eph-source",
                QcDataType::QcEphemerisData,
                QcDataType::QcEphemerisData,
            )
            .unwrap()
            .node(
                "eph-processor#21",
                "eph-processor#2",
                QcDataType::QcEphemerisData,
                QcDataType::QcEphemerisData,
            )
            .unwrap()
            .node(
                "eph-processor#11",
                "eph-processor#1",
                QcDataType::QcEphemerisData,
                QcDataType::QcEphemerisData,
            )
            .unwrap()
            .node(
                "eph-processor#211",
                "eph-processor#21",
                QcDataType::QcEphemerisData,
                QcDataType::QcEphemerisData,
            )
            .unwrap();

        assert!(topology.node_exists("eph-source"));
        assert!(topology.node_exists("eph-processor#1"));
        assert!(topology.node_exists("eph-processor#11"));
        assert!(topology.node_exists("eph-processor#2"));
        assert!(topology.node_exists("eph-processor#21"));
        assert!(topology.node_exists("eph-processor#211"));

        assert_eq!(topology.total_nodes(), 6);

        let (_, fake_channel) = crossbeam_channel::bounded(128);

        let _ = topology.wire(fake_channel).unwrap_or_else(|e| {
            panic!("Failed to create QcPipeline: {}", e);
        });
    }

    //     #[test]
    //     fn topology_designer_1() {

    //         // |--------|
    //         // |source_1|
    //         // |--------|
    //         //      ^----------node_1----node_1_1
    //         //     ^-----------node_2----node_2_1----node_2_1_1

    //         let topology = topology.add_source_node("src_1", QcDataType::QcEphemerisData);

    //         assert_eq!(topology.total_nodes(), 1);

    //         let node_1 = Node::new(
    //             "node_1",
    //             QcDataType::QcEphemerisData,
    //             QcDataType::QcEphemerisData,
    //         )
    //         .with_parent("src_1");

    //         let topology = topology.add_node(node_1).unwrap();

    //         assert_eq!(topology.total_nodes(), 2);

    //         let node_2 = Node::new(
    //             "node_2",
    //             QcDataType::QcEphemerisData,
    //             QcDataType::QcEphemerisData,
    //         )
    //         .with_parent("src_1");

    //         let topology = topology.add_node(node_2).unwrap();

    //         assert_eq!(topology.total_nodes(), 3);

    //         let node_1_1 = Node::new(
    //             "node_1_1",
    //             QcDataType::QcEphemerisData,
    //             QcDataType::QcEphemerisData,
    //         )
    //         .with_parent("node_1");

    //         let topology = topology.add_node(node_1_1).unwrap();

    //         assert_eq!(topology.total_nodes(), 4);

    //         let node_2_1 = Node::new(
    //             "node_2_1",
    //             QcDataType::QcEphemerisData,
    //             QcDataType::QcEphemerisData,
    //         )
    //         .with_parent("node_2");

    //         let topology = topology.add_node(node_2_1).unwrap();

    //         assert_eq!(topology.total_nodes(), 5);

    //         let node_2_1_1 = Node::new(
    //             "node_2_1_1",
    //             QcDataType::QcEphemerisData,
    //             QcDataType::QcEphemerisData,
    //         )
    //         .with_parent("node_2_1");

    //         let topology = topology.add_node(node_2_1_1).unwrap();

    //         assert_eq!(topology.total_nodes(), 6);

    //         let (_, rx) = crossbeam_channel::bounded(128);

    //         let _ = topology.wire(rx).unwrap();
    //     }
}
