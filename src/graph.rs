use self::dominator_tree::DominatorTree;

use fixedbitset::FixedBitSet;
use petgraph::{
    algo::dominators::{self, Dominators},
    graph::{DiGraph, NodeIndex},
    visit::{DfsPostOrder, EdgeRef},
};

use std::collections::{HashMap, HashSet};

type BackEdges = HashMap<NodeIndex, HashSet<NodeIndex>>;

#[derive(Debug)]
pub struct Graph<N, E> {
    pub graph: DiGraph<N, E>,
    pub root: NodeIndex,
    pub return_: NodeIndex,
    pub dominators: Dominators<NodeIndex>,
    pub dominator_tree: DominatorTree,
    pub post_dominators: Dominators<NodeIndex>,
    pub loop_heads: HashSet<NodeIndex>,
    pub back_edges: BackEdges,
}

impl<N, E> Graph<N, E> {
    pub fn new(graph: DiGraph<N, E>, root: NodeIndex, return_: NodeIndex) -> Self {
        let dominators = dominators::simple_fast(&graph, root);
        let dominator_tree = DominatorTree::from_dominators(&graph, root, &dominators);
        let post_dominators = compute_post_dominators(&graph, return_);

        let (loop_heads, back_edges) = find_loop_heads_and_back_edges(&graph, root);

        Self {
            graph,
            root,
            return_,
            dominators,
            dominator_tree,
            post_dominators,
            loop_heads,
            back_edges,
        }
    }

    pub fn recompute_info(&mut self) {
        self.dominators = dominators::simple_fast(&self.graph, self.root);
        self.dominator_tree =
            DominatorTree::from_dominators(&self.graph, self.root, &self.dominators);
        self.post_dominators = compute_post_dominators(&self.graph, self.return_);
        let (loop_heads, back_edges) = find_loop_heads_and_back_edges(&self.graph, self.root);
        self.loop_heads = loop_heads;
        self.back_edges = back_edges;
    }

    pub fn update(
        &mut self,
        node_ndx: NodeIndex,
        to_remove: HashSet<NodeIndex>,
        node: N,
        successors: Vec<(NodeIndex, E)>,
    ) {
        for node in to_remove {
            self.graph.remove_node(node);
        }
        let node_edges = self
            .graph
            .edges_directed(node_ndx, petgraph::Direction::Outgoing)
            .map(|edge| edge.id())
            .collect::<Vec<_>>();
        for edge_ndx in node_edges {
            self.graph.remove_edge(edge_ndx);
        }
        *self.graph.node_weight_mut(node_ndx).unwrap() = node;
        for (succ, edge) in successors.into_iter() {
            self.graph.add_edge(node_ndx, succ, edge);
        }
    }

    pub fn post_dominates(&self, node: NodeIndex, region: &HashSet<NodeIndex>) -> bool {
        region
            .iter()
            .all(|&r| dominates(&self.post_dominators, node, r))
    }

    pub fn successors(&self, node_ndx: &NodeIndex) -> HashSet<NodeIndex> {
        let all: HashSet<_> = self
            .graph
            .neighbors_directed(*node_ndx, petgraph::Direction::Outgoing)
            .collect();

        if let Some(back) = self.back_edges.get(node_ndx) {
            all.difference(back).cloned().collect()
        } else {
            all
        }
    }

    pub fn predecessors(&self, node_ndx: &NodeIndex) -> HashSet<NodeIndex> {
        let all: HashSet<_> = self
            .graph
            .neighbors_directed(*node_ndx, petgraph::Direction::Incoming)
            .collect();

        // In this case, look for any *incoming* back edge, i.e., any `pred → node`
        let back_preds: HashSet<NodeIndex> = self
            .back_edges
            .iter()
            .filter_map(|(pred, targets)| {
                if targets.contains(node_ndx) {
                    Some(*pred)
                } else {
                    None
                }
            })
            .collect();

        all.difference(&back_preds).cloned().collect()
    }

    pub fn dfs_post_order(&self) -> DfsPostOrder<NodeIndex, FixedBitSet> {
        DfsPostOrder::new(&self.graph, self.root)
    }
}

fn dominates(dom: &Dominators<NodeIndex>, a: NodeIndex, mut b: NodeIndex) -> bool {
    while let Some(idom) = dom.immediate_dominator(b) {
        if idom == a {
            return true;
        }
        if idom == b {
            break; // Reached self-dominating root
        }
        b = idom;
    }
    false
}

fn compute_post_dominators<N, E>(
    graph: &DiGraph<N, E>,
    return_: NodeIndex,
) -> Dominators<NodeIndex> {
    // Make an empty, reversed version of the graph
    let graph = petgraph::graph::DiGraph::<(), ()>::from_edges(
        graph.edge_references().map(|e| (e.target(), e.source())),
    );
    dominators::simple_fast(&graph, return_)
}

fn find_loop_heads_and_back_edges<N, E>(
    graph: &DiGraph<N, E>,
    start: NodeIndex,
) -> (HashSet<NodeIndex>, HashMap<NodeIndex, HashSet<NodeIndex>>) {
    pub fn find_recur<N, E>(
        graph: &DiGraph<N, E>,
        visited: &mut HashSet<NodeIndex>,
        path_to_root: &mut Vec<NodeIndex>,
        loop_heads: &mut HashSet<NodeIndex>,
        back_edges: &mut HashMap<NodeIndex, HashSet<NodeIndex>>,
        node: NodeIndex,
    ) {
        if !visited.insert(node) {
            return;
        };

        path_to_root.push(node);
        for edge in graph.edges_directed(node, petgraph::Direction::Outgoing) {
            let target = edge.target();
            if path_to_root
                .iter()
                .any(|ndx| *ndx != node && *ndx == target)
            {
                loop_heads.insert(target);
                back_edges.entry(node).or_default().insert(target);
            }
            find_recur(graph, visited, path_to_root, loop_heads, back_edges, target);
        }
        assert!(node == path_to_root.pop().expect("No seen node to pop"));
    }

    let mut loop_heads = HashSet::new();
    let mut back_edges = HashMap::new();

    find_recur(
        graph,
        &mut HashSet::new(),
        &mut vec![],
        &mut loop_heads,
        &mut back_edges,
        start,
    );

    (loop_heads, back_edges)
}

mod dominator_tree {
    use std::collections::{HashMap, HashSet, VecDeque};

    use petgraph::{
        algo::dominators::Dominators,
        graph::{DiGraph, NodeIndex},
    };

    #[derive(Debug)]
    pub struct DominatorTree(Node);

    #[derive(Debug)]
    pub struct Node {
        value: NodeIndex,
        children: Vec<Node>,
    }

    impl DominatorTree {
        pub fn from_dominators<N, E>(
            graph: &DiGraph<N, E>,
            root: NodeIndex,
            dom: &Dominators<NodeIndex>,
        ) -> DominatorTree {
            fn build_node(
                value: NodeIndex,
                child_map: &mut HashMap<NodeIndex, Vec<NodeIndex>>,
            ) -> Node {
                let children = child_map.remove(&value).unwrap_or(Vec::new());
                let children = children
                    .into_iter()
                    .map(|child| build_node(child, child_map))
                    .collect::<Vec<_>>();
                Node { value, children }
            }

            let mut child_map: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
            let all_nodes: HashSet<NodeIndex> = graph.node_indices().collect();

            for &node in &all_nodes {
                if let Some(idom) = dom.immediate_dominator(node) {
                    // Skip the root
                    if idom != node {
                        child_map.entry(idom).or_default().push(node);
                    }
                }
            }

            // Build tree recursively from root
            let tree = build_node(root, &mut child_map);
            DominatorTree(tree)
        }

        pub fn find(&self, target: NodeIndex) -> Option<&'_ Node> {
            let mut queue = VecDeque::from([&self.0]);

            while let Some(node) = queue.pop_front() {
                if node.value == target {
                    return Some(node);
                };

                node.children
                    .iter()
                    .for_each(|child| queue.push_back(child));
            }

            None
        }
    }

    impl Node {
        pub fn all_children(&self) -> Box<dyn Iterator<Item = NodeIndex> + '_> {
            let iter = self.children.iter().flat_map(|child| {
                let self_iter = std::iter::once(child.value);
                let child_iter = child.all_children();
                self_iter.chain(child_iter)
            });

            Box::new(iter)
        }
    }
}
