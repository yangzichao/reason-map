//! First-run example map. SPEC §8: never present a blank canvas — load a real, pokeable
//! argument so the user immediately sees forward inference, edges, and the adversarial loop.

use crate::db::Db;
use crate::domain::{ChallengeKind, ChallengeTargetKind, EdgeType, NodeStatus, Origin};
use crate::error::AppResult;
use crate::repo;
use crate::repo::challenges::NewChallenge;
use crate::repo::edges::NewEdge;
use crate::repo::nodes::NewNode;

/// Seed an example map iff the library is empty.
pub async fn seed_if_empty(db: &Db) -> AppResult<()> {
    if !repo::maps::list(db).await?.is_empty() {
        return Ok(());
    }
    let map = repo::maps::create(db, "Example: 该不该做 reason-map?").await?;

    let mk = |text: &str, status: NodeStatus, x: f64, y: f64| NewNode {
        map_id: map.id.clone(),
        text: text.to_string(),
        status,
        origin: Origin::User,
        x,
        y,
    };

    let conclusion = repo::nodes::create(
        db,
        mk("我应该投入做 reason-map", NodeStatus::Bet, 360.0, 60.0),
    )
    .await?;
    let gap = repo::nodes::create(
        db,
        mk(
            "现有论证工具没把『推进 + 对抗』做进去,这里有真空白",
            NodeStatus::Assumption,
            120.0,
            240.0,
        ),
    )
    .await?;
    let diff = repo::nodes::create(
        db,
        mk(
            "本地 + Claude 能做出别人做不出的差异化体验",
            NodeStatus::Bet,
            420.0,
            240.0,
        ),
    )
    .await?;
    let need = repo::nodes::create(
        db,
        mk("我自己有强烈的『梳理逻辑』需求", NodeStatus::Fact, 700.0, 240.0),
    )
    .await?;

    for (from, to) in [(&gap, &conclusion), (&diff, &conclusion), (&need, &conclusion)] {
        repo::edges::create(
            db,
            NewEdge {
                map_id: map.id.clone(),
                from_node: from.id.clone(),
                to_node: to.id.clone(),
                edge_type: EdgeType::Support,
                strength: None,
            },
        )
        .await?;
    }

    // A pre-loaded adversarial challenge so the judgment loop is visible on first launch.
    repo::challenges::create(
        db,
        NewChallenge {
            map_id: map.id.clone(),
            target_kind: ChallengeTargetKind::Node,
            target_id: diff.id.clone(),
            kind: ChallengeKind::HiddenAssumption,
            content: "这赌偷偷假设了『复用 Claude 订阅可长期稳定』——一旦认证/条款变动,差异化就塌了。".into(),
        },
    )
    .await?;

    Ok(())
}
