//! Persistence layer. One module per entity (SPEC: small files, grouped by feature).

pub mod challenges;
pub mod chat;
pub mod edges;
pub mod events;
pub mod history;
pub mod maps;
pub mod nodes;
pub mod search;
pub mod settings;

#[cfg(test)]
mod tests;
