pub mod ast;
pub mod graph;
pub mod test;

use std::collections::HashSet;

use petgraph::graph::NodeIndex;

type Structured<N, E> = (
    HashSet<petgraph::prelude::NodeIndex>,
    N,
    Vec<(petgraph::prelude::NodeIndex, E)>,
);

fn main() {
    println!("Hello, world!");

    let (start, end, graph) = test::test_graph();

    let mut graph = graph::Graph::new(graph, start, end);

    println!("Graph: {graph:#?}");

    // Reduce the graph until it is a single node
    while graph.graph.node_count() > 1 {
        graph.recompute_info();

        let mut visited_nodes = vec![];
        let mut post_order = graph.dfs_post_order();
        let back_edges = graph.back_edges.clone();
        while let Some(cur_node) = post_order.next(&graph.graph) {
            if graph.loop_heads.contains(&cur_node) {
                let back_edges = back_edges
                    .iter()
                    .filter(|(_, targets)| targets.iter().any(|target| *target == cur_node))
                    .map(|(ndx, _)| ndx)
                    .collect::<Vec<_>>();
                let (removed_nodes, new_structure, successors) =
                    structure_cyclic(&graph, cur_node, back_edges);
                graph.update(cur_node, removed_nodes, new_structure, successors);
                break;
            }

            let region_nodes = graph
                .dominator_tree
                .find(cur_node)
                .unwrap()
                .all_children()
                .chain(std::iter::once(cur_node))
                .collect::<HashSet<_>>();
            // If this is a dominator node, we are going to treat it as an acyclic region.
            if region_nodes.len() > 1 {
                if let Some(end_nodes) =
                    get_end_nodes(&graph, &visited_nodes, cur_node, &region_nodes, |s| {
                        s.starts_with("n")
                    })
                {
                    let (removed_nodes, new_structure, successors) =
                        structure_acyclic(&graph, cur_node, end_nodes);
                    graph.update(cur_node, removed_nodes, new_structure, successors);
                    break;
                }
            }

            visited_nodes.push(cur_node);
        }
    }
}

//--------------------------------------------------------------------------------------------------
// Acyclic Regions
//--------------------------------------------------------------------------------------------------

fn get_end_nodes<N, E, F>(
    graph: &graph::Graph<N, E>,
    visited_nodes: &[petgraph::prelude::NodeIndex],
    cur_node: NodeIndex,
    region_nodes: &HashSet<petgraph::prelude::NodeIndex>,
    code_node: F,
) -> Option<Vec<NodeIndex>>
where
    F: Fn(&N) -> bool,
{
    println!("region nodes: {:?}", region_nodes);
    let end_nodes = get_end_successor_nodes(graph, &region_nodes);
    if end_nodes.len() <= 1 {
        println!("Single end node, let's go");
        return Some(end_nodes.into_iter().collect());
    };

    let postdom_nodes = get_postdom_nodes(graph, &region_nodes);
    if postdom_nodes.len() <= 1 {
        println!("Single post dom node, let's go");
        return Some(end_nodes.into_iter().collect());
    }

    get_sub_region_end_nodes(graph, code_node, visited_nodes, cur_node, region_nodes)
}

fn get_end_successor_nodes<N, E>(
    graph: &graph::Graph<N, E>,
    region_nodes: &HashSet<NodeIndex>,
) -> HashSet<NodeIndex> {
    let mut end_nodes = HashSet::new();
    for node in region_nodes {
        for succ in graph.successors(node).iter() {
            if !region_nodes.contains(succ) || graph.successors(succ).is_empty() {
                end_nodes.insert(*succ);
            }
        }
    }
    end_nodes
}

fn get_postdom_nodes<N, E>(
    graph: &graph::Graph<N, E>,
    region_nodes: &HashSet<NodeIndex>,
) -> HashSet<NodeIndex> {
    let mut postdom_nodes = HashSet::new();
    for node in region_nodes {
        if let Some(dom) = graph.post_dominators.immediate_dominator(*node) {
            if !region_nodes.contains(&dom) {
                postdom_nodes.insert(dom);
            }
        }
    }
    postdom_nodes
}

pub fn get_sub_region_end_nodes<N, E, F>(
    graph: &graph::Graph<N, E>,
    code_node: F,
    visited_node_in_postorder: &[petgraph::prelude::NodeIndex],
    head: NodeIndex,
    region_nodes: &HashSet<NodeIndex>,
) -> Option<Vec<NodeIndex>>
where
    F: Fn(&N) -> bool,
{
    let mut sub_region_nodes = HashSet::new();
    sub_region_nodes.insert(head);

    for &node in visited_node_in_postorder.iter().rev() {
        if !region_nodes.contains(&node) {
            return None;
        }

        sub_region_nodes.insert(node);

        if graph.post_dominates(node, &sub_region_nodes) {
            let tail_nodes: Vec<NodeIndex> = if graph
                .graph
                .edges_directed(node, petgraph::Direction::Outgoing)
                .count()
                <= 1
            {
                vec![node]
            } else {
                graph.predecessors(&node).into_iter().collect()
            };

            let mut region_nodes = sub_region_nodes.clone();
            region_nodes.extend(tail_nodes.iter().copied());

            if region_nodes
                .iter()
                .filter(|node| code_node(&graph.graph[**node]))
                .count()
                > 1
            {
                return Some(tail_nodes);
            }
        }
    }

    None
}

//--------------------------------------------------------------------------------------------------
// Structuring
//--------------------------------------------------------------------------------------------------

fn structure_cyclic<N: std::fmt::Display, E>(
    graph: &graph::Graph<N, E>,
    cur_node: petgraph::prelude::NodeIndex,
    back_edges: Vec<&petgraph::prelude::NodeIndex>,
) -> Structured<N, E> {
    println!(
        "Handling loop at {} with back edges: {}",
        graph.graph[cur_node],
        back_edges
            .into_iter()
            .map(|node| format!("{}", graph.graph[*node]))
            .collect::<Vec<_>>()
            .join(", ")
    );
    todo!()
}

fn structure_acyclic<N: std::fmt::Display, E>(
    graph: &graph::Graph<N, E>,
    cur_node: petgraph::prelude::NodeIndex,
    end_nodes: Vec<petgraph::prelude::NodeIndex>,
) -> Structured<N, E> {
    println!(
        "Handling acyclic at {} with ends edges: {}",
        graph.graph[cur_node],
        end_nodes
            .into_iter()
            .map(|node| format!("{}", graph.graph[node]))
            .collect::<Vec<_>>()
            .join(", ")
    );
    todo!()
}

/*
    let (reversed_graph, index_map) = reverse_graph(&graph);
    for node in graph.node_indices() {
        print!("{} -> ", graph[node]);
        let edges = graph
            .neighbors_directed(node, petgraph::Direction::Outgoing)
            .map(|node| graph[node].clone())
            .collect::<Vec<_>>()
            .join(", ");
        println!("{edges}");
        let edges = reversed_graph
            .neighbors_directed(node, petgraph::Direction::Outgoing)
            .map(|node| graph[node].clone())
            .collect::<Vec<_>>()
            .join(", ");
        println!("    {edges}");
    }

    let post_dominators = dominators::simple_fast(&reversed_graph, end);
    let post_dominators = MappedDominators {
        inner: post_dominators,
        reverse_map: index_map,
    };

    let regions = compute_regions(&graph, &post_dominators, start);

    for (start, end) in regions {
        println!("Start: {} -- End: {}", graph[start], graph[end]);
    }
}
*/
