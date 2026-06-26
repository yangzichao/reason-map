//! Repository round-trip tests against an in-memory SQLite (schema + FK + FTS exercised).

use crate::db::connect_memory;
use crate::domain::{ChallengeKind, ChallengeStatus, ChallengeTargetKind, EdgeType, NodeStatus};
use crate::repo;
use crate::repo::challenges::NewChallenge;
use crate::repo::edges::NewEdge;
use crate::repo::nodes::NewNode;

#[tokio::test]
async fn full_map_round_trip() {
    let db = connect_memory().await.unwrap();
    let map = repo::maps::create(&db, "T").await.unwrap();

    let a = repo::nodes::create(
        &db,
        NewNode {
            map_id: map.id.clone(),
            text: "premise alpha".into(),
            status: NodeStatus::Bet,
            origin: crate::domain::Origin::User,
            x: 0.0,
            y: 0.0,
        },
    )
    .await
    .unwrap();
    let b = repo::nodes::create(
        &db,
        NewNode {
            map_id: map.id.clone(),
            text: "conclusion beta".into(),
            status: NodeStatus::Open,
            origin: crate::domain::Origin::User,
            x: 0.0,
            y: 0.0,
        },
    )
    .await
    .unwrap();

    repo::edges::create(
        &db,
        NewEdge {
            map_id: map.id.clone(),
            from_node: a.id.clone(),
            to_node: b.id.clone(),
            edge_type: EdgeType::Support,
            strength: None,
        },
    )
    .await
    .unwrap();

    let g = repo::maps::graph(&db, &map.id).await.unwrap();
    assert_eq!(g.nodes.len(), 2);
    assert_eq!(g.edges.len(), 1);

    // FTS over node text.
    let hits = repo::search::search_nodes(&db, &map.id, "alpha").await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, a.id);

    // Status change persists.
    repo::nodes::set_status(&db, &a.id, NodeStatus::Fact).await.unwrap();
    assert!(matches!(
        repo::nodes::get(&db, &a.id).await.unwrap().status,
        NodeStatus::Fact
    ));

    // Challenge + judgment loop.
    let ch = repo::challenges::create(
        &db,
        NewChallenge {
            map_id: map.id.clone(),
            target_kind: ChallengeTargetKind::Node,
            target_id: a.id.clone(),
            kind: ChallengeKind::HiddenAssumption,
            content: "secretly assumes X".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(repo::challenges::pending(&db, &map.id).await.unwrap().len(), 1);

    repo::challenges::judge(&db, &ch.id, ChallengeStatus::Rebutted, Some("no it doesn't"))
        .await
        .unwrap();
    assert_eq!(repo::challenges::pending(&db, &map.id).await.unwrap().len(), 0);
    let hist = repo::challenges::for_target(&db, &a.id).await.unwrap();
    assert_eq!(hist.len(), 1);
    assert_eq!(hist[0].user_note.as_deref(), Some("no it doesn't"));

    // Deleting a node detaches its edges and removes it from FTS.
    repo::nodes::soft_delete(&db, &a.id).await.unwrap();
    let g2 = repo::maps::graph(&db, &map.id).await.unwrap();
    assert_eq!(g2.nodes.len(), 1);
    assert_eq!(g2.edges.len(), 0, "edge detached on node delete");
    assert!(repo::search::search_nodes(&db, &map.id, "alpha").await.unwrap().is_empty());

    // Event log recorded the mutations.
    let events = repo::events::recent(&db, &map.id, 100).await.unwrap();
    assert!(events.iter().any(|e| e.op == "node.create"));
    assert!(events.iter().any(|e| e.op == "node.delete"));
}

#[tokio::test]
async fn promote_is_idempotent_and_gated() {
    let db = connect_memory().await.unwrap();
    let map = repo::maps::create(&db, "T").await.unwrap();
    let target = repo::nodes::create(
        &db,
        NewNode {
            map_id: map.id.clone(),
            text: "target".into(),
            status: NodeStatus::Bet,
            origin: crate::domain::Origin::User,
            x: 0.0,
            y: 0.0,
        },
    )
    .await
    .unwrap();
    let ch = repo::challenges::create(
        &db,
        NewChallenge {
            map_id: map.id.clone(),
            target_kind: ChallengeTargetKind::Node,
            target_id: target.id.clone(),
            kind: ChallengeKind::Rebuttal,
            content: "this is false".into(),
        },
    )
    .await
    .unwrap();

    // Cannot promote while still pending.
    assert!(repo::challenges::promote(&db, &ch.id).await.is_err());

    repo::challenges::judge(&db, &ch.id, ChallengeStatus::Conceded, None)
        .await
        .unwrap();
    let n1 = repo::challenges::promote(&db, &ch.id).await.unwrap();
    // Second promote returns the SAME node, not a duplicate.
    let n2 = repo::challenges::promote(&db, &ch.id).await.unwrap();
    assert_eq!(n1.id, n2.id);

    let g = repo::maps::graph(&db, &map.id).await.unwrap();
    assert_eq!(g.nodes.len(), 2, "target + one promoted node, no duplicate");
    assert_eq!(g.edges.len(), 1, "exactly one rebut edge");
    assert!(matches!(g.edges[0].edge_type, EdgeType::Rebut));
}

#[tokio::test]
async fn undo_reverts_create_and_delete() {
    let db = connect_memory().await.unwrap();
    let map = repo::maps::create(&db, "T").await.unwrap();
    let n = repo::nodes::create(
        &db,
        NewNode {
            map_id: map.id.clone(),
            text: "x".into(),
            status: NodeStatus::Open,
            origin: crate::domain::Origin::User,
            x: 0.0,
            y: 0.0,
        },
    )
    .await
    .unwrap();
    repo::nodes::set_status(&db, &n.id, NodeStatus::Bet).await.unwrap();

    // Undo the status change.
    assert!(repo::history::undo_last(&db, &map.id).await.unwrap());
    assert!(matches!(
        repo::nodes::get(&db, &n.id).await.unwrap().status,
        NodeStatus::Open
    ));
    // Undo the create → node gone from the graph.
    assert!(repo::history::undo_last(&db, &map.id).await.unwrap());
    assert_eq!(repo::maps::graph(&db, &map.id).await.unwrap().nodes.len(), 0);
    // Nothing left to undo.
    assert!(!repo::history::undo_last(&db, &map.id).await.unwrap());
}
