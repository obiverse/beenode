//! HTTP routes for scroll I/O

use axum::{extract::{Path, Query, State}, http::StatusCode, response::IntoResponse, routing::{get, post, put}, Json, Router};
use nine_s_core::namespace::Namespace;
use nine_s_store::Store;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::Node;

// State for Store-based router (legacy)
#[derive(Clone)]
pub struct AppState { pub store: Arc<Store>, pub app_name: String }

impl AppState {
    pub fn new(store: Store, app_name: impl Into<String>) -> Self {
        Self { store: Arc::new(store), app_name: app_name.into() }
    }
}

// State for Node-based router (supports /wallet/* paths)
#[derive(Clone)]
pub struct NodeState { pub node: Arc<Node>, pub app_name: String }

impl NodeState {
    pub fn new(node: Arc<Node>, app_name: impl Into<String>) -> Self {
        Self { node, app_name: app_name.into() }
    }
}

#[derive(Deserialize)]
pub struct ListQuery { #[serde(default = "default_prefix")] prefix: String }
fn default_prefix() -> String { "/".into() }

#[derive(Serialize)]
pub struct ListResponse { paths: Vec<String>, count: usize }

#[derive(Serialize)]
pub struct WriteResponse { key: String, version: u64 }

pub fn create_router(store: Store) -> Router { create_router_with_name(store, "beenode") }

pub fn create_router_with_name(store: Store, app_name: &str) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/scrolls", get(list_scrolls))
        .route("/scroll/*path", get(read_scroll))
        .route("/scroll/*path", post(write_scroll))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(TraceLayer::new_for_http())
        .with_state(AppState::new(store, app_name))
}

/// Create router with Node backend (supports /wallet/*, /nostr/*, etc.)
pub fn create_router_with_node(node: Arc<Node>, app_name: &str) -> Router {
    Router::new()
        .route("/health", get(node_health))
        .route("/scrolls", get(node_list_scrolls))
        .route("/scroll/*path", get(node_read_scroll))
        .route("/scroll/*path", post(node_write_scroll))
        .route("/system/auth/status", get(node_auth_status))
        .route("/system/auth/unlock", put(node_auth_unlock))
        .route("/system/auth/lock", put(node_auth_lock))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(TraceLayer::new_for_http())
        .with_state(NodeState::new(node, app_name))
}

async fn health(State(s): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok", "service": s.app_name}))
}

async fn list_scrolls(State(s): State<AppState>, Query(q): Query<ListQuery>) -> Result<Json<ListResponse>, (StatusCode, String)> {
    let paths = s.store.list(&q.prefix).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(ListResponse { count: paths.len(), paths }))
}

async fn read_scroll(State(s): State<AppState>, Path(path): Path<String>) -> Result<Json<Value>, (StatusCode, String)> {
    let p = if path.starts_with('/') { path } else { format!("/{}", path) };
    match s.store.read(&p) {
        Ok(Some(scroll)) => Ok(Json(serde_json::to_value(scroll).unwrap())),
        Ok(None) => Err((StatusCode::NOT_FOUND, format!("not found: {}", p))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn write_scroll(State(s): State<AppState>, Path(path): Path<String>, Json(data): Json<Value>) -> Result<Json<WriteResponse>, (StatusCode, String)> {
    let p = if path.starts_with('/') { path } else { format!("/{}", path) };
    match s.store.write(&p, data) {
        Ok(scroll) => Ok(Json(WriteResponse { key: scroll.key, version: scroll.metadata.version })),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

// Node-based handlers (support /wallet/*, /nostr/*, etc.)

async fn node_health(State(s): State<NodeState>) -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok", "service": s.app_name}))
}

async fn node_list_scrolls(State(s): State<NodeState>, Query(q): Query<ListQuery>) -> Result<Json<ListResponse>, (StatusCode, String)> {
    let paths = s.node.all(&q.prefix).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(ListResponse { count: paths.len(), paths }))
}

async fn node_read_scroll(State(s): State<NodeState>, Path(path): Path<String>) -> Result<Json<Value>, (StatusCode, String)> {
    let p = if path.starts_with('/') { path } else { format!("/{}", path) };
    match s.node.get(&p) {
        Ok(Some(scroll)) => Ok(Json(serde_json::json!({
            "key": scroll.key,
            "type": scroll.type_,
            "data": scroll.data,
            "metadata": {
                "version": scroll.metadata.version,
                "created_at": scroll.metadata.created_at,
                "updated_at": scroll.metadata.updated_at,
            }
        }))),
        Ok(None) => Err((StatusCode::NOT_FOUND, format!("not found: {}", p))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn node_write_scroll(State(s): State<NodeState>, Path(path): Path<String>, Json(data): Json<Value>) -> Result<Json<WriteResponse>, (StatusCode, String)> {
    let p = if path.starts_with('/') { path } else { format!("/{}", path) };
    match s.node.put(&p, data) {
        Ok(scroll) => Ok(Json(WriteResponse { key: scroll.key, version: scroll.metadata.version })),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

#[derive(Deserialize)]
struct UnlockRequest { pin: String }

#[derive(Serialize)]
struct AuthStatusResponse { locked: bool, initialized: bool }

#[derive(Serialize)]
struct AuthActionResponse { success: bool }

async fn node_auth_status(State(s): State<NodeState>) -> Json<AuthStatusResponse> {
    Json(AuthStatusResponse { locked: s.node.is_locked(), initialized: s.node.is_initialized() })
}

async fn node_auth_unlock(State(s): State<NodeState>, Json(payload): Json<UnlockRequest>) -> Result<Json<AuthActionResponse>, (StatusCode, String)> {
    match s.node.unlock(&payload.pin) {
        Ok(success) => Ok(Json(AuthActionResponse { success })),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

async fn node_auth_lock(State(s): State<NodeState>) -> Result<Json<AuthActionResponse>, (StatusCode, String)> {
    match s.node.lock() {
        Ok(success) => Ok(Json(AuthActionResponse { success })),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}
