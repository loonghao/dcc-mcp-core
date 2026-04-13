//! Standalone `dcc-mcp-server` binary for bridge-mode DCCs.
//!
//! Starts the MCP Streamable HTTP server and (optionally) a WebSocket bridge
//! server so that DCC plugins written in non-Python languages (JavaScript/UXP,
//! C++, C#, GDScript, …) can connect without a local Python installation.
//!
//! ## Simplified deployment
//!
//! ```text
//! DCC Plugin (any language)
//!     ↕  WebSocket :9001  (JSON-RPC 2.0 bridge protocol)
//! dcc-mcp-server  ← this binary, zero deps
//!     ↕  HTTP :8765
//! MCP Client (Claude/Cursor)
//! ```
//!
//! ## Usage
//!
//! ```bash
//! # Auto-discover skills and start both servers
//! dcc-mcp-server
//!
//! # Explicit configuration
//! dcc-mcp-server --mcp-port 8765 --ws-port 9001 --dcc photoshop \
//!   --skill-paths /path/to/skills --server-name "photoshop-mcp"
//!
//! # No WebSocket bridge (MCP HTTP only)
//! dcc-mcp-server --no-bridge
//! ```
//!
//! ## Environment variables
//!
//! | Variable                  | Description                         |
//! |---------------------------|-------------------------------------|
//! | `DCC_MCP_SKILL_PATHS`     | Colon/semicolon-separated skill dirs |
//! | `DCC_MCP_MCP_PORT`        | MCP HTTP server port (default 8765)  |
//! | `DCC_MCP_WS_PORT`         | WebSocket bridge port (default 9001) |
//! | `DCC_MCP_DCC`             | DCC name hint (e.g. "photoshop")     |
//! | `DCC_MCP_SERVER_NAME`     | Server name advertised to MCP client |

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use dcc_mcp_actions::{ActionDispatcher, ActionRegistry};
use dcc_mcp_http::{McpHttpConfig, McpHttpServer};
use dcc_mcp_skills::SkillCatalog;
use dcc_mcp_transport::discovery::file_registry::FileRegistry;
use dcc_mcp_transport::discovery::types::ServiceEntry;
use dcc_mcp_utils::filesystem;

/// Standalone MCP server for bridge-mode DCCs.
#[derive(Debug, Parser)]
#[command(name = "dcc-mcp-server", about, version)]
struct Args {
    /// MCP Streamable HTTP server port.
    #[arg(long, env = "DCC_MCP_MCP_PORT", default_value = "8765")]
    mcp_port: u16,

    /// WebSocket bridge server port (for DCC plugin connections).
    #[arg(long, env = "DCC_MCP_WS_PORT", default_value = "9001")]
    ws_port: u16,

    /// DCC name hint (e.g. "photoshop", "zbrush", "unreal").
    /// Used to resolve DCC-specific skill environment variables.
    #[arg(long, env = "DCC_MCP_DCC", default_value = "")]
    dcc: String,

    /// Additional skill search paths (repeatable).
    #[arg(long, value_name = "PATH", num_args = 1..)]
    skill_paths: Vec<PathBuf>,

    /// Server name advertised to MCP clients.
    #[arg(long, env = "DCC_MCP_SERVER_NAME", default_value = "dcc-mcp-server")]
    server_name: String,

    /// Disable the WebSocket bridge server (MCP HTTP only).
    #[arg(long, default_value = "false")]
    no_bridge: bool,

    /// MCP server host to bind to.
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Directory for the shared service registry (used by dcc-mcp-gateway).
    /// When set, this server registers itself on startup and deregisters on exit,
    /// enabling gateway discovery. Defaults to $TMPDIR/dcc-mcp/.
    /// Set to empty string "" to disable gateway registration.
    #[arg(long, env = "DCC_MCP_REGISTRY_DIR")]
    registry_dir: Option<String>,

    /// DCC application version (e.g. "2024.2"). Reported to the gateway.
    #[arg(long, env = "DCC_MCP_DCC_VERSION")]
    dcc_version: Option<String>,

    /// Currently open scene/project file. Reported to the gateway.
    #[arg(long, env = "DCC_MCP_SCENE")]
    scene: Option<String>,

    /// Heartbeat interval (seconds) for gateway registration. 0 = disabled.
    #[arg(long, env = "DCC_MCP_HEARTBEAT_INTERVAL", default_value = "5")]
    registry_heartbeat_secs: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();

    // ── Collect skill paths ──────────────────────────────────────────────────

    let mut skill_paths: Vec<PathBuf> = args.skill_paths.clone();

    // Add paths from environment variables.
    let env_paths = filesystem::get_skill_paths_from_env();
    skill_paths.extend(env_paths.into_iter().map(PathBuf::from));

    // Add DCC-specific skill paths if a DCC name was provided.
    if !args.dcc.is_empty() {
        let app_paths = filesystem::get_app_skill_paths_from_env(&args.dcc);
        skill_paths.extend(app_paths.into_iter().map(PathBuf::from));
    }

    // Always include the built-in bundled skills.
    if let Ok(bundled) = filesystem::get_skills_dir(None) {
        let bundled_path = PathBuf::from(bundled);
        if bundled_path.exists() {
            skill_paths.push(bundled_path);
        }
    }

    tracing::info!(
        "Skill search paths: {:?}",
        skill_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
    );

    // ── Build registry + catalog ─────────────────────────────────────────────

    let registry = Arc::new(ActionRegistry::new());
    let dispatcher = Arc::new(ActionDispatcher::new((*registry).clone()));
    let catalog = Arc::new(SkillCatalog::new_with_dispatcher(
        registry.clone(),
        dispatcher.clone(),
    ));

    // Discover skills into the catalog so they appear as stubs in tools/list.
    // catalog.discover() scans for SKILL.md files and registers each skill as
    // SkillState::Discovered (unloaded).  Tools from discovered skills show up
    // as lightweight __skill__<name> stubs in tools/list, enabling agents to
    // find and load them on demand without pre-loading all schemas.
    let dcc_hint = if args.dcc.is_empty() {
        None
    } else {
        Some(args.dcc.as_str())
    };
    let extra_dirs: Option<Vec<String>> = if skill_paths.is_empty() {
        None
    } else {
        Some(
            skill_paths
                .iter()
                .filter(|p| p.exists())
                .map(|p| p.display().to_string())
                .collect(),
        )
    };
    let n = catalog.discover(extra_dirs.as_deref(), dcc_hint);
    tracing::info!("Discovered {} skill(s) in catalog", n);

    // ── Start MCP HTTP server ────────────────────────────────────────────────

    let config = McpHttpConfig::new(args.mcp_port)
        .with_name(args.server_name.clone())
        .with_cors();

    let mcp_server = McpHttpServer::with_catalog(registry.clone(), catalog.clone(), config)
        .with_dispatcher(dispatcher.clone());

    let handle = mcp_server.start().await?;

    tracing::info!(
        "MCP HTTP server listening on http://{}:{}",
        args.host,
        handle.port,
    );

    // ── Gateway registration ──────────────────────────────────────────────────
    //
    // If a registry directory is configured (or defaults to $TMPDIR/dcc-mcp/),
    // register this server instance so that dcc-mcp-gateway can discover it.
    // The registration is removed on clean shutdown.

    let registry_dir = args
        .registry_dir
        .as_deref()
        .map(|s| s.to_string())
        .or_else(|| {
            // Default: $TMPDIR/dcc-mcp/
            Some(
                std::env::temp_dir()
                    .join("dcc-mcp")
                    .to_string_lossy()
                    .to_string(),
            )
        });

    let gateway_key = if let Some(ref dir) = registry_dir {
        if dir.is_empty() {
            tracing::info!("Gateway registration disabled (empty registry_dir).");
            None
        } else {
            match FileRegistry::new(dir) {
                Ok(reg) => {
                    let mut entry = ServiceEntry::new(
                        if args.dcc.is_empty() {
                            "unknown"
                        } else {
                            &args.dcc
                        },
                        &args.host,
                        handle.port,
                    );
                    if let Some(ref v) = args.dcc_version {
                        entry.version = Some(v.clone());
                    }
                    if let Some(ref s) = args.scene {
                        entry.scene = Some(s.clone());
                    }
                    entry
                        .metadata
                        .insert("server_name".to_string(), args.server_name.clone());

                    let key = entry.key();
                    match reg.register(entry) {
                        Ok(_) => {
                            tracing::info!(
                                dcc = %args.dcc,
                                port = handle.port,
                                registry = %dir,
                                instance = %key.instance_id,
                                "Registered with gateway registry"
                            );

                            // Heartbeat task: keep registration alive
                            if args.registry_heartbeat_secs > 0 {
                                let reg2 = FileRegistry::new(dir).ok();
                                let key2 = key.clone();
                                let interval = args.registry_heartbeat_secs;
                                tokio::spawn(async move {
                                    let mut tick = tokio::time::interval(
                                        std::time::Duration::from_secs(interval),
                                    );
                                    loop {
                                        tick.tick().await;
                                        if let Some(ref r) = reg2 {
                                            let _ = r.heartbeat(&key2);
                                        }
                                    }
                                });
                            }

                            Some((reg, key))
                        }
                        Err(e) => {
                            tracing::warn!("Failed to register with gateway: {e}");
                            None
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to open registry at '{dir}': {e}");
                    None
                }
            }
        }
    } else {
        None
    };

    // ── Start WebSocket bridge server ────────────────────────────────────────

    if !args.no_bridge {
        let ws_port = args.ws_port;
        let server_name = args.server_name.clone();
        let server_version = env!("CARGO_PKG_VERSION").to_string();

        tokio::spawn(async move {
            run_ws_bridge(ws_port, server_name, server_version).await;
        });
    }

    // ── Wait for shutdown signal ─────────────────────────────────────────────

    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down…");

    // Deregister from gateway registry on clean shutdown.
    if let Some((reg, key)) = gateway_key {
        match reg.deregister(&key) {
            Ok(_) => tracing::info!("Deregistered from gateway registry"),
            Err(e) => tracing::warn!("Failed to deregister from gateway: {e}"),
        }
    }

    handle.shutdown().await;

    Ok(())
}

/// Run the WebSocket bridge server that accepts DCC plugin connections.
async fn run_ws_bridge(port: u16, server_name: String, server_version: String) {
    use tokio::net::TcpListener;

    tracing::info!("WebSocket bridge server listening on ws://127.0.0.1:{port}");

    let listener = match TcpListener::bind(format!("127.0.0.1:{port}")).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to bind WebSocket bridge on port {port}: {e}");
            return;
        }
    };

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let sn = server_name.clone();
                let sv = server_version.clone();
                tracing::debug!("DCC plugin connected from {addr}");
                tokio::spawn(async move {
                    handle_ws_connection(stream, addr, sn, sv).await;
                });
            }
            Err(e) => {
                tracing::warn!("WS bridge accept error: {e}");
            }
        }
    }
}

/// Handle a single WebSocket connection from a DCC plugin.
async fn handle_ws_connection(
    stream: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    server_name: String,
    server_version: String,
) {
    use dcc_mcp_protocols::bridge::{BridgeHelloAck, BridgeMessage};
    use futures_util::{SinkExt, StreamExt};

    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            tracing::warn!("WS handshake failed for {addr}: {e}");
            return;
        }
    };

    let (mut sender, mut receiver) = ws_stream.split();
    let mut greeted = false;

    while let Some(msg_result) = receiver.next().await {
        let raw = match msg_result {
            Ok(tokio_tungstenite::tungstenite::Message::Text(t)) => t.to_string(),
            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
            Ok(_) => continue,
            Err(e) => {
                tracing::debug!("WS receive error from {addr}: {e}");
                break;
            }
        };

        match serde_json::from_str::<BridgeMessage>(&raw) {
            Ok(BridgeMessage::Hello(hello)) => {
                tracing::info!(
                    "DCC hello from {addr}: client={} version={}",
                    hello.client,
                    hello.version
                );
                let ack = BridgeMessage::HelloAck(BridgeHelloAck {
                    server: server_name.clone(),
                    version: server_version.clone(),
                    session_id: uuid::Uuid::new_v4().to_string(),
                });
                let text = serde_json::to_string(&ack).unwrap_or_default();
                let _ = sender
                    .send(tokio_tungstenite::tungstenite::Message::Text(text.into()))
                    .await;
                greeted = true;
            }
            Ok(BridgeMessage::Response(resp)) => {
                tracing::debug!("DCC response (id={}): {:?}", resp.id, resp.result);
            }
            Ok(BridgeMessage::Event(evt)) => {
                tracing::debug!("DCC event: {} {:?}", evt.event, evt.data);
            }
            Ok(BridgeMessage::Disconnect(_)) => {
                tracing::debug!("DCC plugin {addr} sent disconnect");
                break;
            }
            Ok(other) => {
                tracing::debug!("Unhandled bridge message from {addr}: {other:?}");
            }
            Err(e) => {
                let parse_err =
                    BridgeMessage::ParseError(dcc_mcp_protocols::bridge::BridgeParseError {
                        message: e.to_string(),
                    });
                let text = serde_json::to_string(&parse_err).unwrap_or_default();
                let _ = sender
                    .send(tokio_tungstenite::tungstenite::Message::Text(text.into()))
                    .await;
            }
        }
    }

    tracing::debug!("DCC plugin {addr} disconnected (greeted={greeted})");
}
