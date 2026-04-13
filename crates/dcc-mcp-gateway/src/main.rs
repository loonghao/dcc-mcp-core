//! `dcc-mcp-gateway` — Unified MCP gateway for multi-DCC environments.
//!
//! ## Problem it solves
//!
//! In a studio with multiple open DCCs — 3 Maya instances, a Photoshop, a ZBrush —
//! each runs its own `dcc-mcp-server` on a different port.  An MCP client (Claude,
//! Cursor) would need to know all those ports upfront.
//!
//! The gateway provides **one fixed endpoint** that:
//!
//! 1. **Discovers** all running DCC servers via [`FileRegistry`].
//! 2. **Exposes** them as MCP meta-tools so the agent can list and select instances.
//! 3. **Proxies** tool calls to the chosen DCC server (transparent HTTP forwarding).
//!
//! ## Architecture
//!
//! ```text
//! Agent (Claude / Cursor)
//!     │  MCP Streamable HTTP  :8888 (fixed)
//!     ▼
//! dcc-mcp-gateway          ← this binary
//!     │  discovers via FileRegistry ($TMPDIR/dcc-mcp/services.json)
//!     │  proxies via HTTP
//!     ├─▶  Maya-1   :18812  (scene=shot_01.ma)
//!     ├─▶  Maya-2   :18813  (scene=shot_02.ma)
//!     ├─▶  Photoshop :18814  (doc=poster.psd)
//!     └─▶  ZBrush   :18815
//! ```
//!
//! ## MCP tools exposed by the gateway
//!
//! | Tool | Description |
//! |------|-------------|
//! | `list_dcc_instances` | List all live DCC servers (type, port, scene, status) |
//! | `get_dcc_instance` | Get info for a specific instance by id or dcc_type+scene |
//! | `connect_to_dcc` | Return the MCP URL for a specific instance |
//!
//! ## Routing
//!
//! ```
//! POST /mcp                   → gateway meta-tools only
//! POST /mcp/{instance_id}     → proxy to that specific DCC server
//! POST /mcp/dcc/{dcc_type}    → proxy to best instance of that DCC type
//! GET  /instances             → JSON array of all live instances (REST, no MCP)
//! GET  /health                → {"ok": true}
//! ```
//!
//! ## Environment variables
//!
//! | Variable | Description |
//! |----------|-------------|
//! | `DCC_MCP_GATEWAY_PORT` | Gateway port (default 8888) |
//! | `DCC_MCP_REGISTRY_DIR` | FileRegistry directory (default `$TMPDIR/dcc-mcp`) |
//! | `DCC_MCP_STALE_TIMEOUT` | Seconds before an instance is considered stale (default 30) |

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing};
use clap::Parser;
use dcc_mcp_transport::discovery::file_registry::FileRegistry;
use dcc_mcp_transport::discovery::types::{ServiceEntry, ServiceStatus};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::instrument;

// ── CLI ───────────────────────────────────────────────────────────────────────

/// Unified MCP gateway for multi-DCC environments.
#[derive(Debug, Parser)]
#[command(name = "dcc-mcp-gateway", about, version)]
struct Args {
    /// Port for the gateway HTTP server.
    #[arg(long, env = "DCC_MCP_GATEWAY_PORT", default_value = "8888")]
    port: u16,

    /// Host to bind to.
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Directory where DCC servers write their registry file.
    #[arg(long, env = "DCC_MCP_REGISTRY_DIR")]
    registry_dir: Option<String>,

    /// Seconds without a heartbeat before an instance is considered stale.
    #[arg(long, env = "DCC_MCP_STALE_TIMEOUT", default_value = "30")]
    stale_timeout_secs: u64,

    /// Gateway server name reported to MCP clients.
    #[arg(long, default_value = "dcc-mcp-gateway")]
    server_name: String,
}

// ── Shared state ──────────────────────────────────────────────────────────────

#[derive(Clone)]
struct GatewayState {
    registry: Arc<RwLock<FileRegistry>>,
    stale_timeout: Duration,
    server_name: String,
    server_version: String,
    http_client: reqwest::Client,
}

// ── Instance info (serializable view of ServiceEntry) ────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct InstanceInfo {
    instance_id: String,
    dcc_type: String,
    host: String,
    port: u16,
    mcp_url: String,
    status: String,
    scene: Option<String>,
    version: Option<String>,
    metadata: HashMap<String, String>,
    last_heartbeat_ms: u64,
    stale: bool,
}

impl InstanceInfo {
    fn from_entry(entry: &ServiceEntry, stale_timeout: Duration) -> Self {
        let last_heartbeat_ms = entry
            .last_heartbeat
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let stale = entry.is_stale(stale_timeout);
        let mcp_url = format!("http://{}:{}/mcp", entry.host, entry.port);

        Self {
            instance_id: entry.instance_id.to_string(),
            dcc_type: entry.dcc_type.clone(),
            host: entry.host.clone(),
            port: entry.port,
            mcp_url,
            status: entry.status.to_string(),
            scene: entry.scene.clone(),
            version: entry.version.clone(),
            metadata: entry.metadata.clone(),
            last_heartbeat_ms,
            stale,
        }
    }
}

// ── MCP JSON-RPC types ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

impl JsonRpcResponse {
    fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }
    fn error(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(json!({"code": code, "message": message.into()})),
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn resolve_registry_dir(arg: Option<&str>) -> PathBuf {
    if let Some(d) = arg {
        return PathBuf::from(d);
    }
    if let Ok(d) = env::var("DCC_MCP_REGISTRY_DIR") {
        return PathBuf::from(d);
    }
    env::temp_dir().join("dcc-mcp")
}

fn live_instances(registry: &FileRegistry, stale_timeout: Duration) -> Vec<ServiceEntry> {
    registry
        .list_all()
        .into_iter()
        .filter(|e| {
            !e.is_stale(stale_timeout)
                && !matches!(
                    e.status,
                    ServiceStatus::ShuttingDown | ServiceStatus::Unreachable
                )
        })
        .collect()
}

// ── REST handlers ─────────────────────────────────────────────────────────────

/// `GET /health` → `{"ok": true}`
async fn handle_health() -> impl IntoResponse {
    Json(json!({"ok": true}))
}

/// `GET /instances` → JSON array of all live instances.
#[instrument(skip(state))]
async fn handle_instances(State(state): State<GatewayState>) -> impl IntoResponse {
    let registry = state.registry.read().await;
    let instances: Vec<InstanceInfo> = live_instances(&registry, state.stale_timeout)
        .iter()
        .map(|e| InstanceInfo::from_entry(e, state.stale_timeout))
        .collect();
    Json(json!({
        "total": instances.len(),
        "instances": instances,
    }))
}

// ── MCP gateway handler ───────────────────────────────────────────────────────

/// `POST /mcp` — gateway's own MCP endpoint with discovery meta-tools.
#[instrument(skip(state, body))]
async fn handle_gateway_mcp(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    // Support both single request and batch
    if body.is_array() {
        let reqs: Vec<Value> = body.as_array().unwrap().clone();
        let mut responses = Vec::new();
        for req_val in reqs {
            let resp = dispatch_mcp_request(&state, req_val).await;
            responses.push(resp);
        }
        return Json(Value::Array(responses)).into_response();
    }

    let resp = dispatch_mcp_request(&state, body).await;
    let mut response = Json(resp).into_response();
    // Inject a stable gateway session header
    response
        .headers_mut()
        .insert("Mcp-Session-Id", "gateway".parse().unwrap());
    response
}

async fn dispatch_mcp_request(state: &GatewayState, body: Value) -> Value {
    let req: JsonRpcRequest = match serde_json::from_value(body) {
        Ok(r) => r,
        Err(e) => {
            return serde_json::to_value(JsonRpcResponse::error(
                None,
                -32700,
                format!("Parse error: {e}"),
            ))
            .unwrap_or_default();
        }
    };

    let resp = match req.method.as_str() {
        "initialize" => handle_initialize(state, &req),
        "ping" => JsonRpcResponse::success(req.id.clone(), json!({})),
        "notifications/initialized" => JsonRpcResponse::success(req.id.clone(), json!({})),
        "tools/list" => handle_tools_list(state, &req),
        "tools/call" => handle_tools_call(state, &req).await,
        other => {
            JsonRpcResponse::error(req.id.clone(), -32601, format!("Method not found: {other}"))
        }
    };

    serde_json::to_value(resp).unwrap_or_default()
}

fn handle_initialize(state: &GatewayState, req: &JsonRpcRequest) -> JsonRpcResponse {
    JsonRpcResponse::success(
        req.id.clone(),
        json!({
            "protocolVersion": "2025-03-26",
            "capabilities": {"tools": {"listChanged": true}},
            "serverInfo": {
                "name": state.server_name,
                "version": state.server_version,
            },
            "instructions": "DCC Gateway — use list_dcc_instances to see running DCC servers, \
                             connect_to_dcc to get a specific server's MCP URL, \
                             or call tools on a specific DCC via POST /mcp/{instance_id}."
        }),
    )
}

fn handle_tools_list(_state: &GatewayState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let tools = gateway_tools();
    JsonRpcResponse::success(req.id.clone(), json!({"tools": tools, "nextCursor": null}))
}

async fn handle_tools_call(state: &GatewayState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let params = req.params.as_ref().cloned().unwrap_or(json!({}));
    let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = match tool_name {
        "list_dcc_instances" => tool_list_instances(state, &args).await,
        "get_dcc_instance" => tool_get_instance(state, &args).await,
        "connect_to_dcc" => tool_connect_to_dcc(state, &args).await,
        other => Err(format!("Unknown tool: {other}")),
    };

    match result {
        Ok(text) => JsonRpcResponse::success(
            req.id.clone(),
            json!({"content": [{"type": "text", "text": text}], "isError": false}),
        ),
        Err(msg) => JsonRpcResponse::success(
            req.id.clone(),
            json!({"content": [{"type": "text", "text": msg}], "isError": true}),
        ),
    }
}

// ── Gateway meta-tools ────────────────────────────────────────────────────────

async fn tool_list_instances(state: &GatewayState, args: &Value) -> Result<String, String> {
    let dcc_filter = args.get("dcc_type").and_then(|v| v.as_str());

    let registry = state.registry.read().await;
    let mut instances: Vec<InstanceInfo> = live_instances(&registry, state.stale_timeout)
        .iter()
        .filter(|e| dcc_filter.is_none_or(|f| e.dcc_type == f))
        .map(|e| InstanceInfo::from_entry(e, state.stale_timeout))
        .collect();

    // Sort: available first, then by dcc_type, then by port
    instances.sort_by(|a, b| {
        a.status
            .cmp(&b.status)
            .then(a.dcc_type.cmp(&b.dcc_type))
            .then(a.port.cmp(&b.port))
    });

    let result = json!({
        "total": instances.len(),
        "instances": instances,
        "hint": if instances.is_empty() {
            "No live DCC instances found. Start dcc-mcp-server with --registry-dir to register instances."
        } else {
            "Use connect_to_dcc(instance_id=...) to get the MCP URL for a specific instance, \
             or POST /mcp/{instance_id} to route tools/call directly."
        }
    });

    serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
}

async fn tool_get_instance(state: &GatewayState, args: &Value) -> Result<String, String> {
    let registry = state.registry.read().await;
    let all = live_instances(&registry, state.stale_timeout);

    // Match by instance_id (exact or prefix)
    if let Some(id) = args.get("instance_id").and_then(|v| v.as_str()) {
        if let Some(entry) = all.iter().find(|e| {
            let eid = e.instance_id.to_string();
            eid == id || eid.starts_with(id)
        }) {
            let info = InstanceInfo::from_entry(entry, state.stale_timeout);
            return serde_json::to_string_pretty(&info).map_err(|e| e.to_string());
        }
        return Err(format!("Instance '{id}' not found or stale"));
    }

    // Match by dcc_type + optional scene
    if let Some(dcc) = args.get("dcc_type").and_then(|v| v.as_str()) {
        let scene_hint = args.get("scene").and_then(|v| v.as_str());
        let candidates: Vec<&ServiceEntry> = all.iter().filter(|e| e.dcc_type == dcc).collect();

        if candidates.is_empty() {
            return Err(format!("No live instances of dcc_type '{dcc}'"));
        }

        // Scene match if hint provided
        if let Some(hint) = scene_hint {
            if let Some(entry) = candidates
                .iter()
                .find(|e| e.scene.as_deref().unwrap_or("").contains(hint))
            {
                let info = InstanceInfo::from_entry(entry, state.stale_timeout);
                return serde_json::to_string_pretty(&info).map_err(|e| e.to_string());
            }
        }

        // First available
        let info = InstanceInfo::from_entry(candidates[0], state.stale_timeout);
        return serde_json::to_string_pretty(&info).map_err(|e| e.to_string());
    }

    Err("Provide either instance_id or dcc_type".to_string())
}

async fn tool_connect_to_dcc(state: &GatewayState, args: &Value) -> Result<String, String> {
    let registry = state.registry.read().await;
    let all = live_instances(&registry, state.stale_timeout);

    let entry = if let Some(id) = args.get("instance_id").and_then(|v| v.as_str()) {
        all.iter()
            .find(|e| {
                let eid = e.instance_id.to_string();
                eid == id || eid.starts_with(id)
            })
            .cloned()
            .ok_or_else(|| format!("Instance '{id}' not found"))?
    } else if let Some(dcc) = args.get("dcc_type").and_then(|v| v.as_str()) {
        let scene_hint = args.get("scene").and_then(|v| v.as_str());
        let candidates: Vec<&ServiceEntry> = all.iter().filter(|e| e.dcc_type == dcc).collect();
        if candidates.is_empty() {
            return Err(format!("No live instances of dcc_type '{dcc}'"));
        }
        if let Some(hint) = scene_hint {
            candidates
                .iter()
                .find(|e| e.scene.as_deref().unwrap_or("").contains(hint))
                .or(candidates.first())
                .cloned()
                .cloned()
                .ok_or_else(|| "No matching instance".to_string())?
        } else {
            candidates[0].clone()
        }
    } else {
        return Err("Provide instance_id or dcc_type".to_string());
    };

    let mcp_url = format!("http://{}:{}/mcp", entry.host, entry.port);
    let result = json!({
        "instance_id": entry.instance_id.to_string(),
        "dcc_type": entry.dcc_type,
        "mcp_url": mcp_url,
        "proxy_url": format!("http://127.0.0.1:{{gateway_port}}/mcp/{}", entry.instance_id),
        "scene": entry.scene,
        "status": entry.status.to_string(),
        "instructions": format!(
            "Connect your MCP client to: {mcp_url}\n\
             Or use the gateway proxy: POST /mcp/{instance_id}",
            instance_id = entry.instance_id
        )
    });

    serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
}

// ── Proxy handler ─────────────────────────────────────────────────────────────

/// `POST /mcp/{instance_id}` — transparent proxy to a specific DCC server.
#[instrument(skip(state, headers, body))]
async fn handle_proxy_instance(
    State(state): State<GatewayState>,
    Path(instance_id): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let registry = state.registry.read().await;
    let all = registry.list_all();

    let entry = all.iter().find(|e| {
        let eid = e.instance_id.to_string();
        eid == instance_id || eid.starts_with(&instance_id)
    });

    let entry = match entry {
        Some(e) => e.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("Instance '{}' not found", instance_id)})),
            )
                .into_response();
        }
    };
    drop(registry);

    let target_url = format!("http://{}:{}/mcp", entry.host, entry.port);
    proxy_request(&state.http_client, &target_url, headers, body).await
}

/// `POST /mcp/dcc/{dcc_type}` — proxy to best instance of a DCC type.
#[instrument(skip(state, headers, body))]
async fn handle_proxy_dcc_type(
    State(state): State<GatewayState>,
    Path(dcc_type): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let registry = state.registry.read().await;
    let mut candidates: Vec<ServiceEntry> = live_instances(&registry, state.stale_timeout)
        .into_iter()
        .filter(|e| e.dcc_type == dcc_type)
        .collect();
    drop(registry);

    if candidates.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": format!("No live instances of dcc_type '{}'", dcc_type)})),
        )
            .into_response();
    }

    // Pick first available
    candidates.sort_by_key(|e| matches!(e.status, ServiceStatus::Busy) as u8);
    let entry = &candidates[0];
    let target_url = format!("http://{}:{}/mcp", entry.host, entry.port);
    proxy_request(&state.http_client, &target_url, headers, body).await
}

async fn proxy_request(
    client: &reqwest::Client,
    target_url: &str,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let mut req = client.post(target_url).body(body.to_vec());

    // Forward relevant headers (content-type, accept, mcp-session-id)
    for (key, value) in &headers {
        let name = key.as_str().to_lowercase();
        if name == "content-type"
            || name == "accept"
            || name == "mcp-session-id"
            || name == "authorization"
        {
            if let Ok(v) = value.to_str() {
                req = req.header(key.as_str(), v);
            }
        }
    }

    match req.send().await {
        Ok(resp) => {
            let status = StatusCode::from_u16(resp.status().as_u16())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let resp_headers = resp.headers().clone();
            let body_bytes = resp.bytes().await.unwrap_or_default();

            let mut response = Response::new(axum::body::Body::from(body_bytes));
            *response.status_mut() = status;

            // Forward response headers
            for (key, value) in &resp_headers {
                let name = key.as_str().to_lowercase();
                if name == "content-type" || name == "mcp-session-id" || name.starts_with("mcp-") {
                    response.headers_mut().insert(key, value.clone());
                }
            }
            response
        }
        Err(e) => {
            tracing::error!("Proxy request to {target_url} failed: {e}");
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": format!("Upstream DCC server unreachable: {e}")})),
            )
                .into_response()
        }
    }
}

// ── Tool definitions ──────────────────────────────────────────────────────────

fn gateway_tools() -> Value {
    json!([
        {
            "name": "list_dcc_instances",
            "description": "List all running DCC server instances registered with the gateway. \
                           Returns instance IDs, DCC types, ports, open scenes, and status. \
                           Use this to discover what DCCs are available before calling their tools.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "dcc_type": {
                        "type": "string",
                        "description": "Filter by DCC type (e.g. 'maya', 'photoshop'). \
                                       Omit to list all DCC types."
                    }
                }
            }
        },
        {
            "name": "get_dcc_instance",
            "description": "Get detailed information about a specific DCC instance by ID or by type+scene.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "instance_id": {
                        "type": "string",
                        "description": "Instance UUID (or prefix). From list_dcc_instances."
                    },
                    "dcc_type": {
                        "type": "string",
                        "description": "DCC type (e.g. 'maya'). Used when instance_id is not known."
                    },
                    "scene": {
                        "type": "string",
                        "description": "Scene/document name hint for scene-based routing \
                                       (e.g. 'shot_01' matches 'shot_01.ma')."
                    }
                }
            }
        },
        {
            "name": "connect_to_dcc",
            "description": "Get the MCP endpoint URL for a specific DCC instance. \
                           Returns the direct URL (e.g. http://127.0.0.1:18812/mcp) \
                           and a proxy URL through this gateway. \
                           Use the direct URL to connect your MCP client to that DCC, \
                           or use POST /mcp/{instance_id} on this gateway for proxied access.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "instance_id": {
                        "type": "string",
                        "description": "Instance UUID (or prefix)."
                    },
                    "dcc_type": {
                        "type": "string",
                        "description": "DCC type. Selects the best available instance."
                    },
                    "scene": {
                        "type": "string",
                        "description": "Scene hint for scene-based selection."
                    }
                }
            }
        }
    ])
}

// ── Stale cleanup background task ─────────────────────────────────────────────

async fn run_stale_cleanup(registry: Arc<RwLock<FileRegistry>>, stale_timeout: Duration) {
    let mut interval = tokio::time::interval(Duration::from_secs(15));
    loop {
        interval.tick().await;
        let reg = registry.read().await;
        match reg.cleanup_stale(stale_timeout) {
            Ok(n) if n > 0 => tracing::info!("Removed {} stale DCC instance(s)", n),
            Err(e) => tracing::warn!("Stale cleanup error: {e}"),
            _ => {}
        }
    }
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();
    let stale_timeout = Duration::from_secs(args.stale_timeout_secs);

    // ── Open FileRegistry ─────────────────────────────────────────────────

    let registry_dir = resolve_registry_dir(args.registry_dir.as_deref());
    tracing::info!("Registry directory: {}", registry_dir.display());

    let registry = FileRegistry::new(&registry_dir)
        .with_context(|| format!("Failed to open FileRegistry at {}", registry_dir.display()))?;
    let registry = Arc::new(RwLock::new(registry));

    // ── Background stale cleanup ──────────────────────────────────────────

    tokio::spawn(run_stale_cleanup(registry.clone(), stale_timeout));

    // ── Build axum router ─────────────────────────────────────────────────

    let state = GatewayState {
        registry,
        stale_timeout,
        server_name: args.server_name.clone(),
        server_version: env!("CARGO_PKG_VERSION").to_string(),
        http_client: reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?,
    };

    let router = Router::new()
        .route("/health", routing::get(handle_health))
        .route("/instances", routing::get(handle_instances))
        .route("/mcp", routing::post(handle_gateway_mcp))
        .route("/mcp/{instance_id}", routing::post(handle_proxy_instance))
        .route("/mcp/dcc/{dcc_type}", routing::post(handle_proxy_dcc_type))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let bind_addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("Failed to bind to {bind_addr}"))?;

    let actual_addr = listener.local_addr()?;
    tracing::info!("dcc-mcp-gateway listening on http://{actual_addr}");
    tracing::info!("  MCP endpoint:  http://{actual_addr}/mcp");
    tracing::info!("  Instances API: http://{actual_addr}/instances");
    tracing::info!("  Registry dir:  {}", registry_dir.display());

    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("Gateway shutting down…");
        })
        .await?;

    Ok(())
}
