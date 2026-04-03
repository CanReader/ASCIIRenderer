use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    body::Bytes,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Multipart, Path, State,
    },
    http::StatusCode,
    response::{IntoResponse, Json},
};
use dashmap::DashMap;
use futures_util::stream::StreamExt;
use futures_util::sink::SinkExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::interval;
use uuid::Uuid;

use crate::model::{load_fbx, load_gltf, load_obj, Model};
use crate::renderer::{render_frame, RenderParams};

// ─── App state ───────────────────────────────────────────────────────────────

pub struct StoredModel {
    pub model: Arc<Model>,
    pub is_example: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub models: Arc<DashMap<String, StoredModel>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            models: Arc::new(DashMap::new()),
        }
    }
}

// ─── Upload handler ──────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub name: String,
    pub vertex_count: usize,
    pub face_count: usize,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub async fn upload_model(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_name = String::from("model");
    let mut file_data: Option<Bytes> = None;
    let mut ext = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(name) = field.file_name() {
            let n = name.to_string();
            ext = n.rsplit('.').next().unwrap_or("").to_lowercase();
            file_name = n.rsplit('/').next().unwrap_or(&n).to_string();
        }
        if let Ok(data) = field.bytes().await {
            file_data = Some(data);
        }
    }

    let data = match file_data {
        Some(d) => d,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "No file uploaded" })),
            )
                .into_response()
        }
    };

    let model_result = match ext.as_str() {
        "obj" => load_obj(&data, &file_name),
        "gltf" | "glb" => load_gltf(&data, &file_name),
        "fbx" => load_fbx(&data, &file_name),
        _ => Err(format!("Unsupported file type: .{ext}")),
    };

    match model_result {
        Ok(model) => {
            let id = Uuid::new_v4().to_string();
            let resp = UploadResponse {
                id: id.clone(),
                name: model.name.clone(),
                vertex_count: model.vertex_count,
                face_count: model.face_count,
            };
            state.models.insert(id, StoredModel { model: Arc::new(model), is_example: false });
            (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response()
        }
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

// ─── WebSocket render handler ─────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Init {
        model_id: String,
        width: Option<usize>,
        height: Option<usize>,
    },
    UpdateParams {
        params: RenderParams,
    },
    Ping,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    Frame {
        content: String,
        elapsed_ms: u64,
        colors: Option<String>, // hex-encoded RGB, 6 chars per cell, None when no texture
    },
    ModelInfo {
        name: String,
        vertex_count: usize,
        face_count: usize,
    },
    Error {
        message: String,
    },
    Pong,
    Ready,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    use axum::extract::ws::WebSocket;

    let (mut ws_tx, mut ws_rx) = socket.split();

    // ── Handshake: wait for init ──────────────────────────────────────────────
    let init_result = loop {
        let msg = match ws_rx.next().await {
            Some(Ok(Message::Text(t))) => t,
            Some(Ok(Message::Close(_))) | None => return,
            _ => continue,
        };

        let client_msg: ClientMessage = match serde_json::from_str(&msg) {
            Ok(m) => m,
            Err(e) => {
                let err = serde_json::to_string(&ServerMessage::Error {
                    message: format!("Invalid message: {e}"),
                })
                .unwrap();
                let _ = ws_tx.send(Message::Text(err.into())).await;
                continue;
            }
        };

        match client_msg {
            ClientMessage::Init { model_id, width, height } => {
                match state.models.get(&model_id) {
                    Some(m) => {
                        let model_arc = m.model.clone();
                        let info = serde_json::to_string(&ServerMessage::ModelInfo {
                            name: model_arc.name.clone(),
                            vertex_count: model_arc.vertex_count,
                            face_count: model_arc.face_count,
                        })
                        .unwrap();
                        let _ = ws_tx.send(Message::Text(info.into())).await;

                        let mut params = RenderParams::default();
                        if let Some(w) = width { params.width = w; }
                        if let Some(h) = height { params.height = h; }

                        let ready = serde_json::to_string(&ServerMessage::Ready).unwrap();
                        let _ = ws_tx.send(Message::Text(ready.into())).await;

                        break (model_arc, params);
                    }
                    None => {
                        let err = serde_json::to_string(&ServerMessage::Error {
                            message: format!("Model not found: {model_id}"),
                        })
                        .unwrap();
                        let _ = ws_tx.send(Message::Text(err.into())).await;
                    }
                }
            }
            ClientMessage::Ping => {
                let pong = serde_json::to_string(&ServerMessage::Pong).unwrap();
                let _ = ws_tx.send(Message::Text(pong.into())).await;
            }
            _ => {}
        }
    };

    let (model_arc, mut params) = init_result;

    // Channel: receiver task → render loop
    let (param_tx, mut param_rx) = mpsc::channel::<ClientMessage>(32);
    let (close_tx, mut close_rx) = mpsc::channel::<()>(1);

    // Spawn a task to forward incoming WS messages to the render loop
    tokio::spawn(async move {
        while let Some(msg_res) = ws_rx.next().await {
            match msg_res {
                Ok(Message::Text(t)) => {
                    if let Ok(cm) = serde_json::from_str::<ClientMessage>(&t) {
                        if param_tx.send(cm).await.is_err() { break; }
                    }
                }
                Ok(Message::Close(_)) | Err(_) => {
                    let _ = close_tx.send(()).await;
                    break;
                }
                _ => {}
            }
        }
    });

    const TARGET_FPS: f32 = 30.0;
    const FRAME_DT: f32 = 1.0 / TARGET_FPS;
    let mut tick = interval(Duration::from_millis((1000.0 / TARGET_FPS) as u64));

    loop {
        tokio::select! {
            _ = tick.tick() => {}
            _ = close_rx.recv() => return,
        }

        // Drain pending client messages
        loop {
            match param_rx.try_recv() {
                Ok(ClientMessage::UpdateParams { params: new_params }) => {
                    params = new_params;
                }
                Ok(ClientMessage::Ping) => {
                    let pong = serde_json::to_string(&ServerMessage::Pong).unwrap();
                    let _ = ws_tx.send(Message::Text(pong.into())).await;
                }
                Ok(_) | Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => return,
            }
        }

        // Advance auto-rotation
        if params.auto_rotate {
            params.rot_x += params.rotate_speed_x * FRAME_DT;
            params.rot_y += params.rotate_speed_y * FRAME_DT;
            params.rot_z += params.rotate_speed_z * FRAME_DT;
        }

        let model_clone = model_arc.clone();
        let params_clone = params.clone();

        let t0 = Instant::now();
        let (ascii, colors) = tokio::task::spawn_blocking(move || {
            render_frame(&model_clone, &params_clone)
        })
        .await
        .unwrap_or_else(|_| (String::new(), None));

        let elapsed_ms = t0.elapsed().as_millis() as u64;

        let frame_msg = serde_json::to_string(&ServerMessage::Frame {
            content: ascii,
            elapsed_ms,
            colors,
        })
        .unwrap();

        if ws_tx.send(Message::Text(frame_msg.into())).await.is_err() {
            return;
        }
    }
}

// ─── Model list ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub vertex_count: usize,
    pub face_count: usize,
    pub is_example: bool,
}

pub async fn list_models(State(state): State<AppState>) -> Json<Vec<ModelEntry>> {
    let entries: Vec<ModelEntry> = state
        .models
        .iter()
        .map(|entry| {
            let stored = entry.value();
            ModelEntry {
                id: entry.key().clone(),
                name: stored.model.name.clone(),
                vertex_count: stored.model.vertex_count,
                face_count: stored.model.face_count,
                is_example: stored.is_example,
            }
        })
        .collect();
    Json(entries)
}

pub async fn delete_model(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.models.get(&id) {
        Some(stored) if stored.is_example => {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({ "error": "Built-in example models cannot be deleted" })),
            );
        }
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Model not found" })),
            );
        }
        Some(_) => {}
    }
    // Drop the dashmap ref before mutating
    state.models.remove(&id);
    (StatusCode::OK, Json(serde_json::json!({ "deleted": id })))
}
