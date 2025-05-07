use std::collections::HashMap;

#[derive(Debug)]
pub enum RoutingError {}

#[derive(Clone)]
struct Node {
    pub name: String,
}

#[derive(Clone)]
pub struct NodeLayer {
    pub level: usize,
    pub nodes: Vec<Node>,
}

#[derive(Clone)]
pub struct QcPipeline {
    sink: Node,
    layers: Vec<NodeLayer>,
}

impl QcPipeline {
    /// Define a new [QcPipeline]
    pub fn new() -> Self {
        Self {
            sink: Node {
                name: "sink".to_string(),
            },
            layers: Vec::new(),
        }
    }

    pub fn add_node_by_parent_name(&self, parent: &str, node: Node) -> Result<Self, RoutingError> {
        let layer = if parent == "sink" { 1 } else { 2 };

        self.add_node_by_layer(layer, node)
    }

    pub fn add_node_by_layer(&self, layer: usize, node: Node) -> Result<Self, RoutingError> {
        let mut s = self.clone();

        if let Some(layer) = s.layers.iter_mut().find(|k| k.level == layer) {
            layer.nodes.push(node);
        } else {
            s.layers.push(NodeLayer {
                level: layer,
                nodes: vec![node],
            });
        }

        Ok(s)
    }

    pub fn max_depth(&self) -> usize {
        if self.layers.len() == 0 {
            0
        } else {
            self.layers.iter().map(|layer| layer.level).max().unwrap()
        }
    }

    /// Returns total number of nodes, excluding sink!
    pub fn total_nodes(&self) -> usize {
        let mut size = 0;

        for layer in self.layers.iter() {
            size += layer.nodes.len();
        }

        size
    }

    /// Connects every [Node]s described.
    pub fn route(&mut self) {}
}

// let (serializer_tx, entrypoints_rx) = crossbeam_channel::unbounded();
// let (obs_tx, obs_rx) = crossbeam_channel::unbounded();
// let (ephemeris_tx, ephemeris_rx) = crossbeam_channel::unbounded();

// let mut ephemeris_streamer =
//     QcEphemerisStreamer::new("eph-streamer", entrypoints_rx.clone(), ephemeris_tx);

// let mut obs_streamer =
//     QcObservationsStreamer::new("obs-streamer", entrypoints_rx.clone(), obs_tx);

#[cfg(test)]
mod test {
    use super::Node;
    use super::QcPipeline;

    #[test]
    fn pipeline_designer_1() {
        let pipeline = QcPipeline::new();

        // |----|
        // |sink|
        // |-----
        //      ^----------node_1----node_1_1
        //     ^-----------node_2----node_2_1----node_2_2

        assert_eq!(pipeline.max_depth(), 0);
        assert_eq!(pipeline.total_nodes(), 0);

        let node_1 = Node {
            name: "node_1".to_string(),
        };

        let pipeline = pipeline.add_node_by_parent_name("sink", node_1).unwrap();

        assert_eq!(pipeline.max_depth(), 1);
        assert_eq!(pipeline.total_nodes(), 1);

        let node_2 = Node {
            name: "node_2".to_string(),
        };

        let pipeline = pipeline.add_node_by_layer(0, node_2).unwrap();

        assert_eq!(pipeline.max_depth(), 1);
        assert_eq!(pipeline.total_nodes(), 2);

        let node_1_1 = Node {
            name: "node_1_1".to_string(),
        };

        let pipeline = pipeline
            .add_node_by_parent_name("node_1", node_1_1)
            .unwrap();

        assert_eq!(pipeline.max_depth(), 2);
        assert_eq!(pipeline.total_nodes(), 3);

        let node_2_1 = Node {
            name: "node_2_1".to_string(),
        };

        let pipeline = pipeline
            .add_node_by_parent_name("node_2", node_2_1)
            .unwrap();

        assert_eq!(pipeline.max_depth(), 2);
        assert_eq!(pipeline.total_nodes(), 4);

        let node_2_1_1 = Node {
            name: "node_2_1_1".to_string(),
        };

        let pipeline = pipeline
            .add_node_by_parent_name("node_2_1", node_2_1_1)
            .unwrap();

        assert_eq!(pipeline.max_depth(), 3);
        assert_eq!(pipeline.total_nodes(), 5);
    }
}
