use crate::pipeline::{errors::TopologyError, topology::node::Node, types::QcDataType};

use crossbeam_channel::Receiver;

use crate::serializer::data::QcSerializedItem;

use crate::pipeline::QcPipeline;

#[derive(Default, Clone)]
pub struct Topology {
    pub nodes: Vec<Node>,
}

impl Topology {
    /// Define a new [Topology]
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn entrypoint(&self, node: Node) -> Result<Self, TopologyError> {
        let mut s = self.clone();

        if node.parent.is_some() {
            return Err(TopologyError::InternalRoutingError);
        }

        if self.node_exists(&node.name) {
            return Err(TopologyError::NodeAlreadyExists(node.name.to_string()));
        }

        s.nodes.push(node);
        Ok(s)
    }

    pub fn node(&self, node: Node) -> Result<Self, TopologyError> {
        let name = node.name.as_ref();

        let parent_name = node
            .parent
            .as_ref()
            .ok_or(TopologyError::UndefinedParentName)?;

        if !self.node_exists(&parent_name) {
            return Err(TopologyError::ParentDoesNotExist(parent_name.to_string()));
        }

        if self.node_exists(&name) {
            return Err(TopologyError::NodeAlreadyExists(name.to_string()));
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
            if node.parent.is_none() {
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
    pub fn get_source_node(&self) -> Option<&Node> {
        self.nodes.iter().find(|node| node.parent.is_none())
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
            if let Some(parent) = &node.parent {
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

        Ok(QcPipeline {
            serializer_rx,
            topology: self.clone(),
        })
    }
}

#[cfg(test)]
mod test {

    use crate::pipeline::{
        topology::{Node, Topology},
        types::QcDataType,
    };

    #[test]
    fn source_only_topology() {
        let topology = Topology::new()
            .entrypoint(Node::observations_unwrapper("src_1"))
            .unwrap();

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
            .entrypoint(Node::observations_unwrapper("src_1"))
            .unwrap()
            .node(Node::observations_scaler("scaler_1").with_parent("src_1"))
            .unwrap();

        assert!(topology.node_exists("src_1"));
        assert!(topology.node_exists("scaler_1"));
        assert_eq!(topology.total_nodes(), 2);

        // let (_, fake_channel) = crossbeam_channel::bounded(128);

        // let _ = topology.wire(fake_channel).unwrap_or_else(|e| {
        //     panic!("Failed to create QcPipeline: {}", e);
        // });
    }

    #[test]
    fn two_levels_topology() {
        let topology = Topology::new()
            .entrypoint(Node::observations_unwrapper("src_1"))
            .unwrap()
            .node(Node::observations_scaler("scaler_1").with_parent("src_1"))
            .unwrap()
            .node(Node::observations_scaler("scaler_2").with_parent("scaler_1"))
            .unwrap();

        assert!(topology.node_exists("src_1"));
        assert!(topology.node_exists("scaler_1"));
        assert!(topology.node_exists("scaler_2"));

        assert_eq!(topology.total_nodes(), 3);

        // let (_, fake_channel) = crossbeam_channel::bounded(128);

        // let _ = topology.wire(fake_channel).unwrap_or_else(|e| {
        //     panic!("Failed to create QcPipeline: {}", e);
        // });
    }

    #[test]
    fn two_branches_topology() {
        let topology = Topology::new()
            .entrypoint(Node::observations_unwrapper("src_1"))
            .unwrap()
            .node(Node::observations_scaler("scaler_1").with_parent("src_1"))
            .unwrap()
            .node(Node::observations_scaler("scaler_2").with_parent("scaler_1"))
            .unwrap()
            .node(Node::observations_scaler("scaler_3").with_parent("src_1"))
            .unwrap();

        assert!(topology.node_exists("src_1"));
        assert!(topology.node_exists("scaler_1"));
        assert!(topology.node_exists("scaler_2"));
        assert!(topology.node_exists("scaler_3"));

        assert_eq!(topology.total_nodes(), 4);
    }
}
