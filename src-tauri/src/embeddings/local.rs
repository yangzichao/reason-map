//! Real local embedder backed by fastembed (ONNX). Compiled only with the
//! `local-embeddings` feature. Offline after the first model download; data never leaves
//! the machine (SPEC §6).

use std::sync::Mutex;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use super::Embedder;
use crate::error::{AppError, AppResult};

const MODEL_ID: &str = "AllMiniLML6V2";
const DIM: usize = 384;

pub struct LocalEmbedder {
    inner: Mutex<TextEmbedding>,
}

impl LocalEmbedder {
    pub fn new() -> AppResult<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(false),
        )
        .map_err(|e| AppError::Other(format!("fastembed init: {e}")))?;
        Ok(Self {
            inner: Mutex::new(model),
        })
    }
}

impl Embedder for LocalEmbedder {
    fn model_id(&self) -> &str {
        MODEL_ID
    }
    fn dim(&self) -> usize {
        DIM
    }
    fn embed(&self, texts: &[String]) -> AppResult<Vec<Vec<f32>>> {
        let mut model = self
            .inner
            .lock()
            .map_err(|_| AppError::Other("embedder lock poisoned".into()))?;
        let docs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        model
            .embed(docs, None)
            .map_err(|e| AppError::Other(format!("fastembed embed: {e}")))
    }
}
