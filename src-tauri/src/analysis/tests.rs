//! Unit tests for the structural analysis (no DB, pure functions).

use super::*;
use crate::domain::{
    Challenge, ChallengeKind, ChallengeStatus, ChallengeTargetKind, Edge, EdgeType, Map, MapGraph,
    Node, NodeStatus, Origin,
};

fn node(id: &str, status: NodeStatus) -> Node {
    Node {
        id: id.into(),
        map_id: "m".into(),
        text: id.into(),
        status,
        origin: Origin::User,
        x: 0.0,
        y: 0.0,
        created_at: "t".into(),
        updated_at: "t".into(),
        deleted_at: None,
    }
}

fn edge(id: &str, from: &str, to: &str, ty: EdgeType) -> Edge {
    Edge {
        id: id.into(),
        map_id: "m".into(),
        from_node: from.into(),
        to_node: to.into(),
        edge_type: ty,
        strength: None,
        created_at: "t".into(),
        updated_at: "t".into(),
        deleted_at: None,
    }
}

fn graph(nodes: Vec<Node>, edges: Vec<Edge>, challenges: Vec<Challenge>) -> MapGraph {
    MapGraph {
        map: Map {
            id: "m".into(),
            title: "m".into(),
            meta: "{}".into(),
            created_at: "t".into(),
            updated_at: "t".into(),
            deleted_at: None,
        },
        nodes,
        edges,
        challenges,
    }
}

#[test]
fn downstream_and_weak_link() {
    // bet(a) supports b supports c  =>  a is load-bearing with 2 downstream, and a bet => weak.
    let g = graph(
        vec![
            node("a", NodeStatus::Bet),
            node("b", NodeStatus::Open),
            node("c", NodeStatus::Open),
        ],
        vec![
            edge("e1", "a", "b", EdgeType::Support),
            edge("e2", "b", "c", EdgeType::Support),
        ],
        vec![],
    );
    let crit = criticality(&g);
    let a = crit.iter().find(|c| c.node_id == "a").unwrap();
    assert_eq!(a.downstream_count, 2);
    assert!(a.is_load_bearing);
    assert!(a.is_weak_link, "a is a load-bearing bet");

    let c = crit.iter().find(|c| c.node_id == "c").unwrap();
    assert_eq!(c.downstream_count, 0);
    assert!(!c.is_weak_link);
}

#[test]
fn rebuttals_do_not_count_as_dependencies() {
    let g = graph(
        vec![node("a", NodeStatus::Bet), node("b", NodeStatus::Open)],
        vec![edge("e1", "a", "b", EdgeType::Rebut)],
        vec![],
    );
    let crit = criticality(&g);
    let a = crit.iter().find(|c| c.node_id == "a").unwrap();
    assert_eq!(a.downstream_count, 0, "rebut edge is not a dependency");
}

#[test]
fn open_challenges_counted() {
    let ch = Challenge {
        id: "x".into(),
        map_id: "m".into(),
        target_kind: ChallengeTargetKind::Node,
        target_id: "a".into(),
        kind: ChallengeKind::Rebuttal,
        content: "no".into(),
        status: ChallengeStatus::Pending,
        verdict: None,
        user_note: None,
        created_at: "t".into(),
        resolved_at: None,
    };
    let g = graph(vec![node("a", NodeStatus::Open)], vec![], vec![ch]);
    let crit = criticality(&g);
    assert_eq!(crit[0].open_challenges, 1);
}

#[test]
fn detects_circular_reasoning() {
    // a -> b -> a is circular.
    let g = graph(
        vec![node("a", NodeStatus::Open), node("b", NodeStatus::Open)],
        vec![
            edge("e1", "a", "b", EdgeType::Support),
            edge("e2", "b", "a", EdgeType::Support),
        ],
        vec![],
    );
    let circ = circular_nodes(&g);
    assert!(circ.contains("a") && circ.contains("b"));
}

#[test]
fn acyclic_has_no_circular() {
    let g = graph(
        vec![node("a", NodeStatus::Open), node("b", NodeStatus::Open)],
        vec![edge("e1", "a", "b", EdgeType::Support)],
        vec![],
    );
    assert!(circular_nodes(&g).is_empty());
}
