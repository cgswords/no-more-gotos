use petgraph::graph::{DiGraph, NodeIndex};

pub fn test_graph() -> (NodeIndex, NodeIndex, DiGraph<String, ()>) {
    let mut graph: DiGraph<String, ()> = DiGraph::new();
    let entry = graph.add_node("A".to_string());

    let b1 = graph.add_node("b1".to_string());
    let b2 = graph.add_node("b2".to_string());
    let n4 = graph.add_node("n4".to_string());
    let n5 = graph.add_node("n5".to_string());
    let n6 = graph.add_node("n6".to_string());
    let n7 = graph.add_node("n7".to_string());
    let d1 = graph.add_node("d1".to_string());

    let d2 = graph.add_node("d2".to_string());
    let d3 = graph.add_node("d3".to_string());
    let n8 = graph.add_node("n8".to_string());
    let n9 = graph.add_node("n9".to_string());

    let c1 = graph.add_node("c1".to_string());
    let n1 = graph.add_node("n1".to_string());
    let c2 = graph.add_node("c2".to_string());
    let n2 = graph.add_node("n2".to_string());
    let n3 = graph.add_node("n3".to_string());
    let c3 = graph.add_node("c3".to_string());

    graph.add_edge(entry, b1, ());
    // graph.add_edge(entry, g, ());

    graph.add_edge(b1, b2, ());
    graph.add_edge(b1, n4, ());
    graph.add_edge(n4, n5, ());
    graph.add_edge(b2, n5, ());
    graph.add_edge(b2, n6, ());
    graph.add_edge(n5, n7, ());
    graph.add_edge(n6, n7, ());
    graph.add_edge(n7, d1, ());

    graph.add_edge(d1, d2, ());
    graph.add_edge(d1, d3, ());
    graph.add_edge(d2, n8, ());
    graph.add_edge(d2, n9, ());
    graph.add_edge(d3, n8, ());
    graph.add_edge(d3, n9, ());
    graph.add_edge(n8, d1, ());

    graph.add_edge(entry, c1, ());
    graph.add_edge(c1, c2, ());
    graph.add_edge(c1, n1, ());
    graph.add_edge(n1, c1, ());
    graph.add_edge(c2, n3, ());
    graph.add_edge(c2, n2, ());
    graph.add_edge(n3, c3, ());
    graph.add_edge(c3, c1, ());
    graph.add_edge(c3, n9, ());

    (entry, n9, graph)
}
