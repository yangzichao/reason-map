//! reason-map backend. Wires SQLite (source of truth), the Anthropic client, the embedding
//! subsystem, and the full Tauri command surface.

mod analysis;
mod commands;
mod db;
mod domain;
mod embeddings;
mod error;
mod llm;
mod repo;
mod seed;
mod state;

use tauri::Manager;

use state::AppState;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "reason_map=info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Database lives in the OS app-data directory.
            let dir = app
                .path()
                .app_data_dir()
                .expect("no app data dir");
            std::fs::create_dir_all(&dir).expect("create app data dir");
            let db_path = dir.join("reason-map.db");

            let db = tauri::async_runtime::block_on(async {
                let db = db::connect(&db_path).await.expect("db connect/migrate");
                seed::seed_if_empty(&db).await.expect("seed");
                db
            });

            app.manage(AppState::new(db));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // maps + analysis
            commands::maps::list_maps,
            commands::maps::create_map,
            commands::maps::rename_map,
            commands::maps::delete_map,
            commands::maps::export_map,
            commands::maps::load_graph,
            commands::maps::analyze_map,
            commands::maps::detect_circular,
            commands::maps::undo,
            // graph edit
            commands::graph_edit::create_node,
            commands::graph_edit::update_node_text,
            commands::graph_edit::set_node_status,
            commands::graph_edit::set_node_origin,
            commands::graph_edit::move_node,
            commands::graph_edit::delete_node,
            commands::graph_edit::create_edge,
            commands::graph_edit::set_edge_type,
            commands::graph_edit::set_edge_strength,
            commands::graph_edit::delete_edge,
            // challenges
            commands::challenges::list_pending_challenges,
            commands::challenges::challenges_for_target,
            commands::challenges::judge_challenge,
            commands::challenges::promote_challenge,
            // evidence
            commands::evidence::list_evidence,
            commands::evidence::add_evidence,
            commands::evidence::delete_evidence,
            // ai
            commands::ai::forward_inference,
            commands::ai::detect_gap,
            commands::ai::generate_challenge,
            commands::ai::scan_weak_points,
            commands::ai::chat,
            commands::ai::chat_history,
            // misc
            commands::misc::search_nodes,
            commands::misc::semantic_search,
            commands::misc::get_setting,
            commands::misc::set_setting,
            commands::misc::recent_events,
            commands::misc::ai_backend_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running reason-map");
}
