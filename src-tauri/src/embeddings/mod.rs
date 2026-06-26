//! Embedding subsystem. The architecture is fully present regardless of build features:
//! the `Embedder` trait, the dirty-flag re-embed queue, and the semantic-search call site.
//!
//! The actual local model (fastembed/ONNX) is gated behind the `local-embeddings` cargo
//! feature because pulling the ONNX runtime into the default build makes the self-check
//! unverifiable offline (SPEC §6). Without the feature, `default_embedder()` returns a
//! `NullEmbedder`, and semantic search transparently degrades to FTS (SPEC §6 fallback).

use crate::db::Db;
use crate::domain::Node;
use crate::error::AppResult;

/// Produces vector embeddings for node text.
pub trait Embedder: Send + Sync {
    fn model_id(&self) -> &str;
    fn dim(&self) -> usize;
    /// Embed a batch of texts. Returns one vector per input, in order.
    fn embed(&self, texts: &[String]) -> AppResult<Vec<Vec<f32>>>;
    /// Whether this embedder can actually produce vectors (false for the null impl).
    fn is_available(&self) -> bool {
        true
    }
}

/// No-op embedder used when the local model feature is off. Semantic search degrades to FTS.
pub struct NullEmbedder;

impl Embedder for NullEmbedder {
    fn model_id(&self) -> &str {
        "null"
    }
    fn dim(&self) -> usize {
        0
    }
    fn embed(&self, texts: &[String]) -> AppResult<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| Vec::new()).collect())
    }
    fn is_available(&self) -> bool {
        false
    }
}

#[cfg(feature = "local-embeddings")]
mod local;

/// The default embedder for this build.
pub fn default_embedder() -> Box<dyn Embedder> {
    #[cfg(feature = "local-embeddings")]
    {
        match local::LocalEmbedder::new() {
            Ok(e) => return Box::new(e),
            Err(err) => tracing::warn!("local embedder unavailable, falling back to FTS: {err}"),
        }
    }
    Box::new(NullEmbedder)
}

/// Re-embed all nodes whose text changed (dirty = 1). Called opportunistically; a no-op
/// when the embedder is unavailable.
pub async fn reembed_dirty(db: &Db, embedder: &dyn Embedder) -> AppResult<usize> {
    if !embedder.is_available() {
        return Ok(0);
    }
    let dirty: Vec<(String, String)> = sqlx::query_as(
        "SELECT n.id, n.text FROM nodes n \
         LEFT JOIN node_embeddings e ON e.node_id = n.id \
         WHERE n.deleted_at IS NULL AND (e.node_id IS NULL OR e.dirty = 1) LIMIT 256",
    )
    .fetch_all(db)
    .await?;
    if dirty.is_empty() {
        return Ok(0);
    }

    let texts: Vec<String> = dirty.iter().map(|(_, t)| t.clone()).collect();
    let vectors = embedder.embed(&texts)?;
    let model = embedder.model_id().to_string();
    let dim = embedder.dim() as i64;

    for ((node_id, _), vec) in dirty.iter().zip(vectors.iter()) {
        let blob: Vec<u8> = vec.iter().flat_map(|f| f.to_le_bytes()).collect();
        sqlx::query(
            "INSERT INTO node_embeddings (node_id, model, dim, vector, dirty, embedded_at) \
             VALUES (?, ?, ?, ?, 0, ?) \
             ON CONFLICT(node_id) DO UPDATE SET model = excluded.model, dim = excluded.dim, \
             vector = excluded.vector, dirty = 0, embedded_at = excluded.embedded_at",
        )
        .bind(node_id)
        .bind(&model)
        .bind(dim)
        .bind(blob)
        .bind(crate::db::now())
        .execute(db)
        .await?;
    }
    Ok(dirty.len())
}

fn decode_vector(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

/// Semantic search over node embeddings (in-memory cosine — graphs are small). Returns the
/// most similar nodes. Without an available embedder (default build), returns empty so the
/// caller falls back to FTS (SPEC §6).
pub async fn semantic_search(
    db: &Db,
    embedder: &dyn Embedder,
    map_id: &str,
    query: &str,
    limit: usize,
) -> AppResult<Vec<Node>> {
    if !embedder.is_available() {
        return Ok(vec![]);
    }
    reembed_dirty(db, embedder).await?;
    let qv = embedder
        .embed(&[query.to_string()])?
        .into_iter()
        .next()
        .unwrap_or_default();
    if qv.is_empty() {
        return Ok(vec![]);
    }

    let rows: Vec<(String, Vec<u8>)> = sqlx::query_as(
        "SELECT e.node_id, e.vector FROM node_embeddings e \
         JOIN nodes n ON n.id = e.node_id \
         WHERE n.map_id = ? AND n.deleted_at IS NULL AND e.vector IS NOT NULL",
    )
    .bind(map_id)
    .fetch_all(db)
    .await?;

    let mut scored: Vec<(String, f32)> = rows
        .iter()
        .filter_map(|(id, blob)| {
            let v = decode_vector(blob);
            if v.len() == qv.len() {
                Some((id.clone(), cosine(&qv, &v)))
            } else {
                None
            }
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    let mut out = Vec::new();
    for (id, _) in scored {
        if let Ok(n) = crate::repo::nodes::get(db, &id).await {
            out.push(n);
        }
    }
    Ok(out)
}
