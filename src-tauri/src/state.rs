//! Shared application state, managed by Tauri and injected into every command.

use crate::db::Db;
use crate::embeddings::{self, Embedder};
use crate::llm::cli::ClaudeCli;

pub struct AppState {
    pub db: Db,
    pub llm: ClaudeCli,
    pub embedder: Box<dyn Embedder>,
}

impl AppState {
    pub fn new(db: Db) -> Self {
        Self {
            db,
            llm: ClaudeCli::new(),
            embedder: embeddings::default_embedder(),
        }
    }
}
