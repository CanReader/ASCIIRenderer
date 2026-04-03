use ascii_renderer::api::{delete_model, list_models, upload_model, ws_handler, AppState, StoredModel};
use ascii_renderer::examples;
use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
    Router,
};
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "ascii_renderer=debug,tower_http=info".parse().unwrap()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState::new();

    // Pre-load built-in procedural example models
    for (name, model) in examples::builtin_models() {
        let id = uuid::Uuid::new_v4().to_string();
        tracing::info!("Loading built-in model: {} (id={})", name, id);
        state.models.insert(id, StoredModel {
            model: std::sync::Arc::new(model),
            is_example: true,
        });
    }

    // Pre-load OBJ asset models embedded at compile time
    for (name, model) in examples::asset_models() {
        let id = uuid::Uuid::new_v4().to_string();
        tracing::info!("Loading asset model: {} (id={})", name, id);
        state.models.insert(id, StoredModel {
            model: std::sync::Arc::new(model),
            is_example: true,
        });
    }

    // Pre-load GLB asset models embedded at compile time
    for (name, model) in examples::asset_models_gltf() {
        let id = uuid::Uuid::new_v4().to_string();
        tracing::info!("Loading GLB asset model: {} (id={})", name, id);
        state.models.insert(id, StoredModel {
            model: std::sync::Arc::new(model),
            is_example: true,
        });
    }

    // In production the binary is run from the project root, so frontend/dist is
    // a sibling of the backend/ directory.  Override with STATIC_DIR if needed.
    let static_dir = std::env::var("STATIC_DIR")
        .unwrap_or_else(|_| "frontend/dist".to_string());

    let serve_dir = ServeDir::new(&static_dir)
        .fallback(ServeFile::new(format!("{static_dir}/index.html")));

    let app = Router::new()
        .route("/api/upload", post(upload_model)
            .layer(DefaultBodyLimit::max(200 * 1024 * 1024))) // 200 MB
        .route("/api/models", get(list_models))
        .route("/api/models/:id", delete(delete_model))
        .route("/ws/render", get(ws_handler))
        .fallback_service(serve_dir)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("ASCII Renderer listening on http://0.0.0.0:3000  (static: {static_dir})");
    axum::serve(listener, app).await.unwrap();
}
