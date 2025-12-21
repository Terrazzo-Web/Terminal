use std::sync::Arc;
use std::sync::Mutex;

use terrazzo::prelude::*;

use super::graph::Graph;
use super::graph::Node;

#[derive(Clone, Debug)]
pub struct GraphX(GraphXInner);

type GraphXInner = XSignal<Ptr<Mutex<Graph>>>;

impl Graph {
    pub fn open(content: &str) -> serde_json::Result<GraphX> {
        let graph = serde_json::from_str(content)?;
        Ok(XSignal::new("graph", graph).into())
    }
}

impl GraphX {
    pub fn keys(&self) -> XSignal<Vec<i32>> {
        self.view("graph-keys", |graph| {
            graph.lock().unwrap().nodes.keys().cloned().collect()
        })
    }

    pub fn get(&self, i: i32) -> XSignal<Option<Arc<Node>>> {
        self.derive(
            format!("node-{i}"),
            move |graph| graph.lock().unwrap().nodes.get(&i).cloned(),
            move |graph, node| {
                let graph = graph.clone();
                if let Some(node) = node {
                    graph.lock().unwrap().nodes.insert(i, node.clone());
                } else {
                    graph.lock().unwrap().nodes.remove(&i);
                }
                Some(graph)
            },
        )
    }
}

mod conversions {
    use std::ops::Deref;

    use super::GraphX;
    use super::GraphXInner;

    impl Deref for GraphX {
        type Target = GraphXInner;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl AsRef<GraphXInner> for GraphX {
        fn as_ref(&self) -> &GraphXInner {
            &self.0
        }
    }

    impl From<GraphXInner> for GraphX {
        fn from(value: GraphXInner) -> Self {
            Self(value)
        }
    }

    impl From<GraphX> for GraphXInner {
        fn from(value: GraphX) -> Self {
            value.0
        }
    }
}
