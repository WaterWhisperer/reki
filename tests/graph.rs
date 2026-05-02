use reki::graph::Graph;
use reki::model::CommitId;

fn id(value: &str) -> CommitId {
    CommitId::new(value)
}

#[test]
fn graph_renders_linear_history_without_extra_lanes() {
    let mut graph = Graph::default();

    assert_eq!(graph.next_row(&id("a"), &[id("b")]).text, "* ");
    assert_eq!(graph.next_row(&id("b"), &[id("c")]).text, "* ");
    assert_eq!(graph.next_row(&id("c"), &[]).text, "* ");
}

#[test]
fn graph_tracks_branch_and_merge_lanes() {
    let mut graph = Graph::default();

    assert_eq!(graph.next_row(&id("a"), &[id("b"), id("c")]).text, "* ");
    assert_eq!(graph.next_row(&id("b"), &[id("d")]).text, "* | ");
    assert_eq!(graph.next_row(&id("c"), &[id("d")]).text, "| * ");
    assert_eq!(graph.next_row(&id("d"), &[id("e")]).text, "* | ");
    assert_eq!(graph.next_row(&id("e"), &[]).text, "* ");
}
