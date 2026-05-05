//! Server configuration.

use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;

/// How the server and gateway HTTP listeners are driven.
///
/// Fixes **issue #303** — under PyO3-embedded interpreters (Maya on Windows),
/// `tokio::spawn` onto a multi-threaded runtime that no longer has an active
/// driver can cause background accept loops (specifically the gateway
/// listener) to be starved of scheduling time. The per-instance listener
/// survives because its accept loop is "warmed up" during the initial
/// `block_on`, but the gateway listener — spawned via an extra `tokio::spawn`
/// + `tokio::join!` layer — never gets its turn.
///
/// `ServerSpawnMode::Dedicated` avoids the failure mode entirely by running
/// each HTTP listener on its own OS thread that owns a `current_thread`
/// Tokio runtime. That thread is scheduled by the OS, not by a shared
/// worker pool, and cannot be starved by a hanging block_on elsewhere.
///
/// | Mode | When to use | Behaviour |
/// |------|-------------|-----------|
/// | `Ambient`   | Standalone binary (`dcc-mcp-server`, library tests) | Spawns `axum::serve` onto the caller's Tokio runtime via `tokio::spawn`. |
/// | `Dedicated` | Python bindings (`PyMcpHttpServer`) / embedded DCC hosts | Each listener gets its own OS thread + `current_thread` runtime. Immune to PyO3 worker starvation. |
///
/// Defaults: `Ambient`. The Python bindings override this to `Dedicated`
/// automatically when constructing `McpHttpServer` via `PyMcpHttpServer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ServerSpawnMode {
    /// Spawn listeners as background tasks on the caller's Tokio runtime.
    /// Correct for `#[tokio::main]` binaries that keep a thread in the
    /// runtime for the process lifetime.
    #[default]
    Ambient,

    /// Spawn each listener on a dedicated OS thread with its own
    /// `current_thread` runtime. Correct for PyO3-embedded interpreters
    /// where the parent runtime's worker pool cannot be relied upon after
    /// `block_on` returns.
    Dedicated,
}

/// What [`McpHttpServer::start`](crate::McpHttpServer::start) does with rows
/// that the previous process left in `Pending` / `Running` after a crash or
/// restart (issue #567).
///
/// | Variant | Behaviour |
/// |---------|-----------|
/// | [`JobRecoveryPolicy::Drop`]    | Each in-flight row is rewritten to [`JobStatus::Interrupted`](crate::job::JobStatus::Interrupted) with `error = "server restart"`. Clients re-subscribing after reconnect see one clean terminal transition. **This is today's behaviour and the default.** |
/// | [`JobRecoveryPolicy::Requeue`] | Reserved for a future release that persists the original tool arguments alongside the `jobs` row. Until that lands the variant is **accepted but treated as `Drop`** — the server logs a `WARN` at startup so operators know the requested policy is not yet active, but startup itself never fails. The accepted-but-degraded contract gives DCC adapters (`dcc-mcp-maya`, `dcc-mcp-houdini`) a stable knob to plumb through today without forcing a config-shape break when the real implementation lands. |
///
/// String form (used by the Python binding and the upcoming `--job-recovery`
/// CLI flag): `"drop"` / `"requeue"`. Defaults to `Drop`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JobRecoveryPolicy {
    /// Rewrite every `Pending` / `Running` row to `Interrupted` on startup.
    /// Always safe; never re-runs a partially-applied tool.
    #[default]
    Drop,
    /// Reserved policy: would re-submit idempotent in-flight jobs from the
    /// persisted spec. Accepted today but treated as [`Self::Drop`] with a
    /// `WARN` log at startup until tool-arg persistence lands.
    Requeue,
}

impl JobRecoveryPolicy {
    /// Lower-case wire identifier used by docs, the Python binding, and the
    /// `--job-recovery` CLI flag.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Drop => "drop",
            Self::Requeue => "requeue",
        }
    }

    /// Parse the wire identifier. `&str` is matched case-insensitively to
    /// be tolerant of env-var plumbing (`DCC_MCP_*_JOB_RECOVERY=Requeue`).
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "drop" => Ok(Self::Drop),
            "requeue" => Ok(Self::Requeue),
            other => Err(format!(
                "unknown job_recovery policy {other:?}; expected \"drop\" or \"requeue\""
            )),
        }
    }
}

// ── Sub-configs (issue #764) ──────────────────────────────────────────────────

/// Gateway election and routing settings extracted from [`McpHttpConfig`] (issue #764).
///
/// Groups every knob that controls how this process competes for the gateway
/// role and how the elected gateway fans out requests to DCC instances.
///
/// Note: the `dcc_mcp_gateway` crate has its own `GatewayConfig` (used for the
/// internal gateway runner).  This type is the HTTP-layer view of the same
/// settings and is named `HttpGatewayConfig` to avoid a name collision when
/// both are in scope.
#[derive(Debug, Clone)]
pub struct HttpGatewayConfig {
    /// Gateway port to compete for. `0` disables the gateway.
    pub port: u16,

    /// Shared `FileRegistry` directory. `None` uses a system temp dir.
    pub registry_dir: Option<PathBuf>,

    /// Seconds without a heartbeat before an instance is considered stale. Default: 30.
    pub stale_timeout_secs: u64,

    /// Heartbeat interval in seconds. `0` disables the heartbeat task. Default: 5.
    pub heartbeat_secs: u64,

    /// Per-backend request timeout (ms) for the gateway fan-out. Default: 120_000.
    pub backend_timeout_ms: u64,

    /// Gateway timeout (ms) for async-dispatch `tools/call`. Default: 60_000.
    pub async_dispatch_timeout_ms: u64,

    /// Gateway timeout (ms) for the wait-for-terminal passthrough mode. Default: 600_000.
    pub wait_terminal_timeout_ms: u64,

    /// TTL (seconds) for the gateway per-job routing cache. Default: 86_400.
    pub route_ttl_secs: u64,

    /// Per-session ceiling on concurrent live routes in the routing cache. Default: 1_000.
    pub max_routes_per_session: u64,

    /// Allow instances with `dcc_type == "unknown"` to be visible via the gateway. Default: false.
    pub allow_unknown_tools: bool,

    /// Adapter package version for version-aware gateway election (issue maya#137).
    pub adapter_version: Option<String>,

    /// DCC type the adapter is bound to; drives the gateway election tiebreaker. Default: None.
    pub adapter_dcc: Option<String>,

    /// Emit Cursor-safe gateway prompt names (`i_<id8>__<escaped>`). Default: true.
    pub cursor_safe_tool_names: bool,
}

impl Default for HttpGatewayConfig {
    fn default() -> Self {
        Self {
            port: 0,
            registry_dir: None,
            stale_timeout_secs: 30,
            heartbeat_secs: 5,
            backend_timeout_ms: 120_000,
            async_dispatch_timeout_ms: 60_000,
            wait_terminal_timeout_ms: 600_000,
            route_ttl_secs: 60 * 60 * 24,
            max_routes_per_session: 1_000,
            allow_unknown_tools: false,
            adapter_version: None,
            adapter_dcc: None,
            cursor_safe_tool_names: true,
        }
    }
}

/// Deferred/bridge/host queue capacities and send timeout (issue #715, #764).
#[derive(Debug, Clone)]
pub struct QueueConfig {
    /// Capacity of the HTTP → `DccExecutor` mpsc channel. Default: 16.
    pub deferred_queue_depth: usize,

    /// Capacity of the `DeferredExecutor` → `host_bridge` mpsc channel. Default: 16.
    pub bridge_queue_depth: usize,

    /// Capacity of the host-side `QueueDispatcher`. `0` = unbounded (default).
    pub host_queue_depth: usize,

    /// How long an HTTP worker blocks on a full executor channel before
    /// returning `QueueOverloaded`. Default: 2_000 ms.
    pub send_timeout_ms: u64,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            deferred_queue_depth: 16,
            bridge_queue_depth: 16,
            host_queue_depth: 0,
            send_timeout_ms: 2_000,
        }
    }
}

/// Prometheus metrics settings (issue #331, #764).
#[derive(Debug, Clone, Default)]
pub struct TelemetryConfig {
    /// Enable the `/metrics` Prometheus endpoint. Default: false.
    pub enable_prometheus: bool,

    /// Optional HTTP Basic auth guard for `/metrics`. Default: None.
    pub prometheus_basic_auth: Option<(String, String)>,
}

/// Workflow subsystem settings (issue #348, #764).
#[derive(Debug, Clone, Default)]
pub struct WorkflowConfig {
    /// Enable the built-in `workflows.*` tools. Default: false.
    pub enable_workflows: bool,
}

/// Cron + webhook scheduler settings (issue #352, #764).
#[derive(Debug, Clone, Default)]
pub struct SchedulerConfig {
    /// Enable the scheduler subsystem. Default: false.
    pub enable_scheduler: bool,

    /// Directory holding `*.schedules.yaml` files. Consulted only when `enable_scheduler` is true.
    pub schedules_dir: Option<PathBuf>,
}

/// Feature flag booleans (issue #764).
///
/// Groups the opt-in capability toggles that do not belong to a more specific
/// domain: lazy-actions fast-path, bare tool names, MCP primitives, and
/// connection-level caching.
#[derive(Debug, Clone)]
pub struct FeatureFlags {
    /// Enable the opt-in lazy-actions meta-tools. Default: false.
    pub lazy_actions: bool,

    /// Publish skill-scoped tools under their bare action name when no collision exists. Default: true.
    pub bare_tool_names: bool,

    /// Advertise the MCP Resources primitive. Default: true.
    pub enable_resources: bool,

    /// Advertise the MCP Prompts primitive. Default: true.
    pub enable_prompts: bool,

    /// Expose `artefact://` resources. Default: false.
    pub enable_artefact_resources: bool,

    /// Emit `$/dcc.jobUpdated` and `$/dcc.workflowUpdated` SSE channels. Default: true.
    pub enable_job_notifications: bool,

    /// Enable connection-scoped tool-list caching. Default: true.
    pub enable_tool_cache: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            lazy_actions: false,
            bare_tool_names: true,
            enable_resources: true,
            enable_prompts: true,
            enable_artefact_resources: false,
            enable_job_notifications: true,
            enable_tool_cache: true,
        }
    }
}

/// Session / TTL / auth / job persistence settings (issue #764).
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Maximum concurrent SSE sessions. Default: 100.
    pub max_sessions: usize,

    /// Idle session TTL in seconds. `0` disables automatic eviction. Default: 3600.
    pub session_ttl_secs: u64,

    /// Path to a SQLite database for persisting tracked jobs. Default: None.
    pub job_storage_path: Option<PathBuf>,

    /// What to do with rows left in `Pending` / `Running` after a restart. Default: Drop.
    pub job_recovery: JobRecoveryPolicy,

    /// Best-effort safety net: shutdown when a `McpServerHandle` is dropped. Default: false.
    pub shutdown_on_drop: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_sessions: 100,
            session_ttl_secs: 3_600,
            job_storage_path: None,
            job_recovery: JobRecoveryPolicy::Drop,
            shutdown_on_drop: false,
        }
    }
}

// ── McpHttpConfig ─────────────────────────────────────────────────────────────

/// Configuration for [`McpHttpServer`](crate::McpHttpServer).
///
/// `McpHttpConfig` is a thin aggregate of cohesive sub-configs (issue #764).
/// All fields remain `pub` at the top level so existing code and the Python
/// wrapper (`PyMcpHttpConfig`) continue to compile unchanged.  The sub-config
/// view methods (`gateway_config()`, `queue_config()`, etc.) provide a typed
/// window into each domain without moving any fields.
#[derive(Debug, Clone)]
pub struct McpHttpConfig {
    /// Port to listen on. Default: 8765.
    pub port: u16,

    /// IP address to bind. Default: 127.0.0.1 (localhost only, per MCP security spec).
    pub host: IpAddr,

    /// MCP endpoint path. Default: `/mcp`.
    pub endpoint_path: String,

    /// Server name reported in MCP `initialize` response.
    pub server_name: String,

    /// Server version reported in MCP `initialize` response.
    pub server_version: String,

    /// Request timeout in milliseconds. Default: 30_000.
    pub request_timeout_ms: u64,

    /// Whether to enable CORS for browser-based MCP clients. Default: false.
    pub enable_cors: bool,

    /// How listener tasks are driven. See [`ServerSpawnMode`]. Default: Ambient.
    pub spawn_mode: ServerSpawnMode,

    /// Maximum time (ms) to wait when self-probing a freshly bound listener. Default: 200.
    pub self_probe_timeout_ms: u64,

    /// DCC application type (e.g. `"maya"`, `"blender"`).
    pub dcc_type: Option<String>,

    /// DCC application version (e.g. `"2025.1"`).
    pub dcc_version: Option<String>,

    /// Currently open scene/file. Improves routing accuracy.
    pub scene: Option<String>,

    /// Arbitrary instance metadata recorded in FileRegistry.
    pub instance_metadata: HashMap<String, String>,

    /// Capabilities declared by the DCC adapter hosting this server (issue #354).
    pub declared_capabilities: Vec<String>,

    // ── SessionConfig fields ───────────────────────────────────────────────
    /// Maximum concurrent SSE sessions. Default: 100.
    pub max_sessions: usize,

    /// Idle session TTL in seconds. Default: 3600. Set to 0 to disable eviction.
    pub session_ttl_secs: u64,

    /// Path to a SQLite database file for persisting tracked jobs (issue #328).
    pub job_storage_path: Option<PathBuf>,

    /// What to do with rows left in `Pending` / `Running` after a restart (issue #567).
    pub job_recovery: JobRecoveryPolicy,

    /// Best-effort safety net: shutdown when `McpServerHandle` is dropped. Default: false.
    pub shutdown_on_drop: bool,

    // ── GatewayConfig fields ──────────────────────────────────────────────
    /// Gateway port to compete for. `0` disables gateway. Default: 0.
    pub gateway_port: u16,

    /// Shared `FileRegistry` directory. `None` uses a system temp dir.
    pub registry_dir: Option<PathBuf>,

    /// Seconds without a heartbeat before an instance is stale. Default: 30.
    pub stale_timeout_secs: u64,

    /// Heartbeat interval in seconds. `0` disables heartbeat. Default: 5.
    pub heartbeat_secs: u64,

    /// Per-backend gateway fan-out timeout (ms). Default: 120_000.
    pub backend_timeout_ms: u64,

    /// Gateway timeout (ms) for async-dispatch `tools/call` (issue #321). Default: 60_000.
    pub gateway_async_dispatch_timeout_ms: u64,

    /// Gateway timeout (ms) for the wait-for-terminal passthrough (issue #321). Default: 600_000.
    pub gateway_wait_terminal_timeout_ms: u64,

    /// TTL (seconds) for the gateway per-job routing cache (issue #322). Default: 86_400.
    pub gateway_route_ttl_secs: u64,

    /// Per-session ceiling on concurrent live gateway routes (issue #322). Default: 1_000.
    pub gateway_max_routes_per_session: u64,

    /// Allow instances with `dcc_type == "unknown"` via the gateway (issue #555). Default: false.
    pub allow_unknown_tools: bool,

    /// Adapter package version for version-aware gateway election (issue maya#137).
    pub adapter_version: Option<String>,

    /// DCC type the adapter is bound to; drives the gateway election tiebreaker (issue maya#137).
    pub adapter_dcc: Option<String>,

    /// Emit Cursor-safe gateway prompt names (issue #656). Default: true.
    pub gateway_cursor_safe_tool_names: bool,

    // ── TelemetryConfig fields ─────────────────────────────────────────────
    /// Enable the Prometheus `/metrics` endpoint (issue #331). Default: false.
    pub enable_prometheus: bool,

    /// Optional HTTP Basic auth guard for `/metrics` (issue #331). Default: None.
    pub prometheus_basic_auth: Option<(String, String)>,

    // ── WorkflowConfig fields ──────────────────────────────────────────────
    /// Enable the built-in `workflows.*` tools (issue #348). Default: false.
    pub enable_workflows: bool,

    // ── SchedulerConfig fields ─────────────────────────────────────────────
    /// Enable the cron + webhook scheduler subsystem (issue #352). Default: false.
    pub enable_scheduler: bool,

    /// Directory holding `*.schedules.yaml` files (issue #352).
    pub schedules_dir: Option<PathBuf>,

    // ── FeatureFlags fields ────────────────────────────────────────────────
    /// Enable the opt-in lazy-actions meta-tools (#254). Default: false.
    pub lazy_actions: bool,

    /// Publish skill-scoped tools under their bare action name (#307). Default: true.
    pub bare_tool_names: bool,

    /// Advertise the MCP Resources primitive (issue #350). Default: true.
    pub enable_resources: bool,

    /// Advertise the MCP Prompts primitive (issues #351, #355). Default: true.
    pub enable_prompts: bool,

    /// Expose `artefact://` resources (issue #349). Default: false.
    pub enable_artefact_resources: bool,

    /// Emit `$/dcc.jobUpdated` / `$/dcc.workflowUpdated` SSE channels (issue #326). Default: true.
    pub enable_job_notifications: bool,

    /// Enable connection-scoped tool-list caching (issue #438). Default: true.
    pub enable_tool_cache: bool,

    // ── QueueConfig fields ─────────────────────────────────────────────────
    /// Capacity of the HTTP → `DccExecutor` mpsc channel (issue #715). Default: 16.
    pub deferred_queue_depth: usize,

    /// Capacity of the `DeferredExecutor` → `host_bridge` mpsc channel (issue #715). Default: 16.
    pub bridge_queue_depth: usize,

    /// Capacity of the host-side `QueueDispatcher` (issue #715). `0` = unbounded.
    pub host_queue_depth: usize,

    /// How long an HTTP worker blocks on a full channel before returning `QueueOverloaded` (issue #715). Default: 2_000 ms.
    pub queue_send_timeout_ms: u64,
}

impl McpHttpConfig {
    /// Create a config with the given port and sensible defaults.
    pub fn new(port: u16) -> Self {
        Self {
            port,
            host: IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            endpoint_path: "/mcp".to_string(),
            server_name: "dcc-mcp".to_string(),
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            request_timeout_ms: 30_000,
            enable_cors: false,
            spawn_mode: ServerSpawnMode::Ambient,
            self_probe_timeout_ms: 200,
            dcc_type: None,
            dcc_version: None,
            scene: None,
            instance_metadata: HashMap::new(),
            declared_capabilities: Vec::new(),
            // SessionConfig
            max_sessions: 100,
            session_ttl_secs: 3_600,
            job_storage_path: None,
            job_recovery: JobRecoveryPolicy::Drop,
            shutdown_on_drop: false,
            // GatewayConfig
            gateway_port: 0,
            registry_dir: None,
            stale_timeout_secs: 30,
            heartbeat_secs: 5,
            backend_timeout_ms: 120_000,
            gateway_async_dispatch_timeout_ms: 60_000,
            gateway_wait_terminal_timeout_ms: 600_000,
            gateway_route_ttl_secs: 60 * 60 * 24,
            gateway_max_routes_per_session: 1_000,
            allow_unknown_tools: false,
            adapter_version: None,
            adapter_dcc: None,
            // #656: default to Cursor-safe gateway prompt names because
            // breakage is silent on that client — prompts would simply
            // never appear.
            gateway_cursor_safe_tool_names: true,
            // TelemetryConfig
            enable_prometheus: false,
            prometheus_basic_auth: None,
            // WorkflowConfig
            enable_workflows: false,
            // SchedulerConfig
            enable_scheduler: false,
            schedules_dir: None,
            // FeatureFlags
            lazy_actions: false,
            bare_tool_names: true,
            enable_resources: true,
            enable_prompts: true,
            enable_artefact_resources: false,
            enable_job_notifications: true,
            enable_tool_cache: true,
            // QueueConfig — default to the pre-#715 behaviour (16 / 16 / unbounded)
            // so existing callers are unaffected until they opt into bounded mode.
            deferred_queue_depth: 16,
            bridge_queue_depth: 16,
            host_queue_depth: 0,
            queue_send_timeout_ms: 2_000,
        }
    }

    // ── Sub-config view methods (issue #764) ──────────────────────────────

    /// Return a snapshot of the gateway-related fields as an [`HttpGatewayConfig`].
    pub fn gateway_config(&self) -> HttpGatewayConfig {
        HttpGatewayConfig {
            port: self.gateway_port,
            registry_dir: self.registry_dir.clone(),
            stale_timeout_secs: self.stale_timeout_secs,
            heartbeat_secs: self.heartbeat_secs,
            backend_timeout_ms: self.backend_timeout_ms,
            async_dispatch_timeout_ms: self.gateway_async_dispatch_timeout_ms,
            wait_terminal_timeout_ms: self.gateway_wait_terminal_timeout_ms,
            route_ttl_secs: self.gateway_route_ttl_secs,
            max_routes_per_session: self.gateway_max_routes_per_session,
            allow_unknown_tools: self.allow_unknown_tools,
            adapter_version: self.adapter_version.clone(),
            adapter_dcc: self.adapter_dcc.clone(),
            cursor_safe_tool_names: self.gateway_cursor_safe_tool_names,
        }
    }

    /// Return a snapshot of the queue-related fields as a [`QueueConfig`].
    pub fn queue_config(&self) -> QueueConfig {
        QueueConfig {
            deferred_queue_depth: self.deferred_queue_depth,
            bridge_queue_depth: self.bridge_queue_depth,
            host_queue_depth: self.host_queue_depth,
            send_timeout_ms: self.queue_send_timeout_ms,
        }
    }

    /// Return a snapshot of the telemetry-related fields as a [`TelemetryConfig`].
    pub fn telemetry_config(&self) -> TelemetryConfig {
        TelemetryConfig {
            enable_prometheus: self.enable_prometheus,
            prometheus_basic_auth: self.prometheus_basic_auth.clone(),
        }
    }

    /// Return a snapshot of the workflow-related fields as a [`WorkflowConfig`].
    pub fn workflow_config(&self) -> WorkflowConfig {
        WorkflowConfig {
            enable_workflows: self.enable_workflows,
        }
    }

    /// Return a snapshot of the scheduler-related fields as a [`SchedulerConfig`].
    pub fn scheduler_config(&self) -> SchedulerConfig {
        SchedulerConfig {
            enable_scheduler: self.enable_scheduler,
            schedules_dir: self.schedules_dir.clone(),
        }
    }

    /// Return a snapshot of the feature-flag fields as [`FeatureFlags`].
    pub fn feature_flags(&self) -> FeatureFlags {
        FeatureFlags {
            lazy_actions: self.lazy_actions,
            bare_tool_names: self.bare_tool_names,
            enable_resources: self.enable_resources,
            enable_prompts: self.enable_prompts,
            enable_artefact_resources: self.enable_artefact_resources,
            enable_job_notifications: self.enable_job_notifications,
            enable_tool_cache: self.enable_tool_cache,
        }
    }

    /// Return a snapshot of the session/TTL/auth/job fields as a [`SessionConfig`].
    pub fn session_config(&self) -> SessionConfig {
        SessionConfig {
            max_sessions: self.max_sessions,
            session_ttl_secs: self.session_ttl_secs,
            job_storage_path: self.job_storage_path.clone(),
            job_recovery: self.job_recovery,
            shutdown_on_drop: self.shutdown_on_drop,
        }
    }

    // ── Builder methods ───────────────────────────────────────────────────

    /// Builder: stamp the adapter package version onto the gateway
    /// sentinel for version-aware election (issue maya#137).
    pub fn with_adapter_version(mut self, version: impl Into<String>) -> Self {
        self.adapter_version = Some(version.into());
        self
    }

    /// Builder: declare the DCC type this adapter is bound to so the
    /// gateway election can prefer real DCCs over generic standalone
    /// servers (issue maya#137).
    pub fn with_adapter_dcc(mut self, dcc: impl Into<String>) -> Self {
        self.adapter_dcc = Some(dcc.into());
        self
    }

    /// Builder: choose the gateway tool-name wire form (issue #656).
    ///
    /// When `true` (the default), the gateway emits Cursor-safe names
    /// of the form `i_<id8>__<escaped_tool>` that survive the stricter
    /// `^[A-Za-z0-9_]+$` regex enforced by Cursor and several other
    /// MCP clients. When `false`, the gateway falls back to the
    /// pre-#656 SEP-986 dotted form `<id8>.<tool>`; use this only when
    /// you need diagnostic parity with a single-instance server that
    /// publishes dotted names directly.
    ///
    /// ```
    /// use dcc_mcp_http::McpHttpConfig;
    ///
    /// let cfg = McpHttpConfig::new(0)
    ///     .with_gateway_cursor_safe_tool_names(false);
    /// assert!(!cfg.gateway_cursor_safe_tool_names);
    /// ```
    pub fn with_gateway_cursor_safe_tool_names(mut self, enabled: bool) -> Self {
        self.gateway_cursor_safe_tool_names = enabled;
        self
    }

    /// Builder: attach context/provenance metadata to the FileRegistry row.
    pub fn with_instance_metadata<I, K, V>(mut self, metadata: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.instance_metadata = metadata
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();
        self
    }

    /// Builder: enable the scheduler subsystem and point at a directory
    /// of `*.schedules.yaml` files (issue #352).
    pub fn with_scheduler(mut self, dir: impl Into<PathBuf>) -> Self {
        self.enable_scheduler = true;
        self.schedules_dir = Some(dir.into());
        self
    }

    /// Builder: persist tracked jobs in a SQLite database at `path`
    /// (issue #328).
    ///
    /// Requires the `job-persist-sqlite` Cargo feature; otherwise
    /// [`McpHttpServer::start`](crate::McpHttpServer::start) fails
    /// with a descriptive error at startup.
    pub fn with_job_storage_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.job_storage_path = Some(path.into());
        self
    }

    /// Builder: choose how the next [`McpHttpServer::start`](crate::McpHttpServer::start)
    /// reacts to in-flight rows persisted by a previous run (issue #567).
    ///
    /// See [`JobRecoveryPolicy`] for the supported variants. Today
    /// `Requeue` is accepted but degrades to `Drop` with a `WARN` log;
    /// the contract is reserved so adapter code (`dcc-mcp-maya`,
    /// `dcc-mcp-houdini`) can wire the knob through now and pick up
    /// the real behaviour transparently when tool-arg persistence
    /// lands.
    pub fn with_job_recovery(mut self, policy: JobRecoveryPolicy) -> Self {
        self.job_recovery = policy;
        self
    }

    /// Builder: declare the DCC capabilities this host provides (issue #354).
    ///
    /// Replaces any existing capability list. Pass freeform string tags like
    /// `"usd"`, `"scene.mutate"`, `"filesystem.read"`.
    pub fn with_declared_capabilities<I, S>(mut self, caps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.declared_capabilities = caps.into_iter().map(Into::into).collect();
        self
    }

    /// Builder: enable the built-in `workflows.*` tools (issue #348).
    ///
    /// See [`Self::enable_workflows`] for the full contract.
    pub fn with_workflows(mut self) -> Self {
        self.enable_workflows = true;
        self
    }

    /// Builder: enable the lazy-actions fast-path (#254).
    ///
    /// Surfaces `list_actions`, `describe_action` and `call_action` as
    /// core MCP tools. Useful for agents whose context budget cannot
    /// afford paging through every skill's full schema.
    pub fn with_lazy_actions(mut self) -> Self {
        self.lazy_actions = true;
        self
    }

    /// Builder: force the legacy `<skill>.<action>` form even when bare
    /// names would be unique (#307).
    ///
    /// Useful for downstream clients that hard-coded the prefixed shape and
    /// cannot be updated in lock-step with the server.
    pub fn without_bare_tool_names(mut self) -> Self {
        self.bare_tool_names = false;
        self
    }

    /// Returns the full socket address string, e.g. `127.0.0.1:8765`.
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Builder: set server name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.server_name = name.into();
        self
    }

    /// Builder: set server version.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.server_version = version.into();
        self
    }

    /// Builder: allow all interfaces (0.0.0.0). Use with caution.
    pub fn with_all_interfaces(mut self) -> Self {
        self.host = IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED);
        self
    }

    /// Builder: enable CORS (for browser clients).
    pub fn with_cors(mut self) -> Self {
        self.enable_cors = true;
        self
    }

    /// Builder: set request timeout.
    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.request_timeout_ms = ms;
        self
    }

    /// Builder: set the idle session TTL. 0 disables background eviction.
    pub fn with_session_ttl_secs(mut self, secs: u64) -> Self {
        self.session_ttl_secs = secs;
        self
    }

    /// Builder: enable gateway competition on the given port.
    ///
    /// The first process to bind this port becomes the gateway. Others run as
    /// plain DCC instances and register themselves in the shared `FileRegistry`.
    pub fn with_gateway(mut self, port: u16) -> Self {
        self.gateway_port = port;
        self
    }

    /// Builder: set the shared FileRegistry directory.
    pub fn with_registry_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.registry_dir = Some(dir.into());
        self
    }

    /// Builder: set the DCC application type (e.g. `"maya"`).
    pub fn with_dcc_type(mut self, dcc_type: impl Into<String>) -> Self {
        self.dcc_type = Some(dcc_type.into());
        self
    }

    /// Builder: select the listener spawn strategy (issue #303).
    ///
    /// Defaults to [`ServerSpawnMode::Ambient`]. Use
    /// [`ServerSpawnMode::Dedicated`] for PyO3-embedded callers so that
    /// listener accept loops are not starved of scheduling time when the
    /// parent runtime has no active driver thread.
    pub fn with_spawn_mode(mut self, mode: ServerSpawnMode) -> Self {
        self.spawn_mode = mode;
        self
    }

    /// Builder: override the per-backend gateway fan-out timeout (issue #314).
    ///
    /// Default: `10_000` ms. Raise this for workflows whose backend tools
    /// legitimately run longer than 10 seconds (scene import, simulation
    /// bake, large USD composition) so the gateway does not short-circuit
    /// them with a transport timeout error.
    pub fn with_backend_timeout_ms(mut self, ms: u64) -> Self {
        self.backend_timeout_ms = ms;
        self
    }

    /// Builder: override the gateway's async-dispatch timeout (issue #321).
    ///
    /// See [`Self::gateway_async_dispatch_timeout_ms`] for the full contract.
    pub fn with_gateway_async_dispatch_timeout_ms(mut self, ms: u64) -> Self {
        self.gateway_async_dispatch_timeout_ms = ms;
        self
    }

    /// Builder: override the gateway wait-for-terminal timeout (issue #321).
    ///
    /// See [`Self::gateway_wait_terminal_timeout_ms`] for the full contract.
    pub fn with_gateway_wait_terminal_timeout_ms(mut self, ms: u64) -> Self {
        self.gateway_wait_terminal_timeout_ms = ms;
        self
    }

    /// Builder: override the gateway's routing-cache TTL (issue #322).
    ///
    /// See [`Self::gateway_route_ttl_secs`] for the full contract.
    pub fn with_gateway_route_ttl_secs(mut self, secs: u64) -> Self {
        self.gateway_route_ttl_secs = secs;
        self
    }

    /// Builder: override the gateway's per-session route cap (issue #322).
    ///
    /// See [`Self::gateway_max_routes_per_session`] for the full contract.
    pub fn with_gateway_max_routes_per_session(mut self, cap: u64) -> Self {
        self.gateway_max_routes_per_session = cap;
        self
    }

    /// Builder: disable the connection-scoped tool-list cache (issue #438).
    ///
    /// By default the cache is enabled. Use this to force every `tools/list`
    /// call to rebuild the full list from scratch (e.g. for debugging or
    /// when tool definitions are mutated externally and no registry
    /// generation bump occurs).
    pub fn without_tool_cache(mut self) -> Self {
        self.enable_tool_cache = false;
        self
    }

    /// Builder: set the deferred-executor queue capacity (issue #715).
    ///
    /// Threaded down into [`crate::executor::DeferredExecutor::new`] at
    /// startup. Setting this to `0` is a logic bug (it would create an
    /// executor that can never accept a task) so the runtime clamps to
    /// `1` at server-start time.
    pub fn with_deferred_queue_depth(mut self, depth: usize) -> Self {
        self.deferred_queue_depth = depth;
        self
    }

    /// Builder: set the host-bridge queue capacity (issue #715).
    ///
    /// Threaded down into
    /// [`crate::host_bridge::dispatcher_to_executor_handle_with_capacity`].
    /// `0` degrades to [`crate::host_bridge::DEFAULT_BRIDGE_QUEUE_DEPTH`]
    /// so misconfigured env-vars cannot silently disable backpressure.
    pub fn with_bridge_queue_depth(mut self, depth: usize) -> Self {
        self.bridge_queue_depth = depth;
        self
    }

    /// Builder: set the host-side `QueueDispatcher` capacity (issue #715).
    ///
    /// `0` (the default) keeps the dispatcher unbounded — the historical
    /// behaviour. Non-zero values activate the
    /// [`dcc_mcp_host::DispatchError::QueueOverloaded`] path once the
    /// queue hits capacity.
    pub fn with_host_queue_depth(mut self, depth: usize) -> Self {
        self.host_queue_depth = depth;
        self
    }

    /// Builder: set the send-timeout applied to the bounded channels
    /// (issue #715).
    pub fn with_queue_send_timeout_ms(mut self, ms: u64) -> Self {
        self.queue_send_timeout_ms = ms;
        self
    }

    /// Load the queue-stack knobs from the environment (issue #715).
    ///
    /// Honours four optional env-vars:
    /// - `MCP_QUEUE_DEFERRED_CAP`
    /// - `MCP_QUEUE_BRIDGE_CAP`
    /// - `MCP_QUEUE_DISPATCHER_CAP`
    /// - `MCP_QUEUE_SEND_TIMEOUT_MS`
    ///
    /// Unset or unparsable values leave the existing field untouched
    /// so a typo never silently flips the cap.
    pub fn apply_queue_env_overrides(mut self) -> Self {
        fn load_usize(key: &str) -> Option<usize> {
            std::env::var(key).ok().and_then(|v| v.trim().parse().ok())
        }
        fn load_u64(key: &str) -> Option<u64> {
            std::env::var(key).ok().and_then(|v| v.trim().parse().ok())
        }
        if let Some(v) = load_usize("MCP_QUEUE_DEFERRED_CAP") {
            self.deferred_queue_depth = v.max(1);
        }
        if let Some(v) = load_usize("MCP_QUEUE_BRIDGE_CAP") {
            self.bridge_queue_depth = v;
        }
        if let Some(v) = load_usize("MCP_QUEUE_DISPATCHER_CAP") {
            self.host_queue_depth = v;
        }
        if let Some(v) = load_u64("MCP_QUEUE_SEND_TIMEOUT_MS") {
            self.queue_send_timeout_ms = v;
        }
        self
    }
}

impl Default for McpHttpConfig {
    fn default() -> Self {
        Self::new(8765)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue #567: the policy enum defaults to `Drop` so existing callers
    /// inherit today's behaviour without touching their config.
    #[test]
    fn job_recovery_default_is_drop() {
        let cfg = McpHttpConfig::new(8765);
        assert_eq!(cfg.job_recovery, JobRecoveryPolicy::Drop);
    }

    /// Issue #567: the builder takes the policy by value and round-trips
    /// to the same wire identifier the Python binding exposes.
    #[test]
    fn job_recovery_builder_round_trips() {
        let cfg = McpHttpConfig::new(8765).with_job_recovery(JobRecoveryPolicy::Requeue);
        assert_eq!(cfg.job_recovery, JobRecoveryPolicy::Requeue);
        assert_eq!(cfg.job_recovery.as_str(), "requeue");
    }

    /// Issue #567: env-var plumbing (`DCC_MCP_*_JOB_RECOVERY=Requeue`) and
    /// the Python setter share the same case-insensitive parser.
    #[test]
    fn job_recovery_parse_is_case_insensitive() {
        for raw in ["drop", "Drop", "DROP", "  drop  "] {
            assert_eq!(JobRecoveryPolicy::parse(raw), Ok(JobRecoveryPolicy::Drop));
        }
        for raw in ["requeue", "Requeue", "REQUEUE"] {
            assert_eq!(
                JobRecoveryPolicy::parse(raw),
                Ok(JobRecoveryPolicy::Requeue)
            );
        }
    }

    /// Issue #567: unknown policies surface a descriptive error that
    /// names the rejected value and the accepted set.
    #[test]
    fn job_recovery_parse_rejects_unknown() {
        let err = JobRecoveryPolicy::parse("retry").unwrap_err();
        assert!(err.contains("retry"), "error message: {err}");
        assert!(err.contains("drop"), "error message: {err}");
        assert!(err.contains("requeue"), "error message: {err}");
    }

    /// Issue #715: the three queue caps default to the pre-#715
    /// values so existing callers are unaffected.
    #[test]
    fn queue_caps_default_to_pre_715_values() {
        let cfg = McpHttpConfig::new(8765);
        assert_eq!(cfg.deferred_queue_depth, 16);
        assert_eq!(cfg.bridge_queue_depth, 16);
        assert_eq!(cfg.host_queue_depth, 0);
        assert_eq!(cfg.queue_send_timeout_ms, 2_000);
    }

    /// Issue #715: env-var overrides are applied and bad values are
    /// silently ignored (typo safety).
    #[test]
    fn queue_env_overrides_apply_and_ignore_typos() {
        // Use a unique key prefix so parallel tests do not collide.
        // SAFETY: we only touch the four #715 env-vars here; no other
        // test reads them.
        unsafe {
            std::env::set_var("MCP_QUEUE_DEFERRED_CAP", "32");
            std::env::set_var("MCP_QUEUE_BRIDGE_CAP", "64");
            std::env::set_var("MCP_QUEUE_DISPATCHER_CAP", "128");
            std::env::set_var("MCP_QUEUE_SEND_TIMEOUT_MS", "bogus");
        }
        let cfg = McpHttpConfig::new(0).apply_queue_env_overrides();
        assert_eq!(cfg.deferred_queue_depth, 32);
        assert_eq!(cfg.bridge_queue_depth, 64);
        assert_eq!(cfg.host_queue_depth, 128);
        assert_eq!(
            cfg.queue_send_timeout_ms, 2_000,
            "unparsable value is ignored (typo safety)"
        );
        unsafe {
            std::env::remove_var("MCP_QUEUE_DEFERRED_CAP");
            std::env::remove_var("MCP_QUEUE_BRIDGE_CAP");
            std::env::remove_var("MCP_QUEUE_DISPATCHER_CAP");
            std::env::remove_var("MCP_QUEUE_SEND_TIMEOUT_MS");
        }
    }

    /// Issue #764: sub-config view methods reflect the flat fields.
    #[test]
    fn sub_config_views_reflect_flat_fields() {
        let cfg = McpHttpConfig::new(8765);

        let gw = cfg.gateway_config();
        assert_eq!(gw.port, cfg.gateway_port);
        assert_eq!(gw.stale_timeout_secs, cfg.stale_timeout_secs);
        assert_eq!(gw.backend_timeout_ms, cfg.backend_timeout_ms);
        assert_eq!(
            gw.cursor_safe_tool_names,
            cfg.gateway_cursor_safe_tool_names
        );

        let q = cfg.queue_config();
        assert_eq!(q.deferred_queue_depth, cfg.deferred_queue_depth);
        assert_eq!(q.bridge_queue_depth, cfg.bridge_queue_depth);
        assert_eq!(q.send_timeout_ms, cfg.queue_send_timeout_ms);

        let tel = cfg.telemetry_config();
        assert_eq!(tel.enable_prometheus, cfg.enable_prometheus);

        let wf = cfg.workflow_config();
        assert_eq!(wf.enable_workflows, cfg.enable_workflows);

        let sched = cfg.scheduler_config();
        assert_eq!(sched.enable_scheduler, cfg.enable_scheduler);

        let ff = cfg.feature_flags();
        assert_eq!(ff.lazy_actions, cfg.lazy_actions);
        assert_eq!(ff.bare_tool_names, cfg.bare_tool_names);
        assert_eq!(ff.enable_resources, cfg.enable_resources);

        let sess = cfg.session_config();
        assert_eq!(sess.max_sessions, cfg.max_sessions);
        assert_eq!(sess.session_ttl_secs, cfg.session_ttl_secs);
        assert_eq!(sess.job_recovery, cfg.job_recovery);
    }

    /// Issue #764: sub-config view methods reflect custom (non-default) flat values so
    /// a write-then-read round-trip confirms every field is correctly mapped.
    #[test]
    fn sub_config_views_reflect_custom_values() {
        let cfg = McpHttpConfig::new(9000)
            .with_gateway(7000)
            .with_backend_timeout_ms(50_000)
            .with_gateway_async_dispatch_timeout_ms(30_000)
            .with_gateway_wait_terminal_timeout_ms(120_000)
            .with_gateway_route_ttl_secs(3600)
            .with_gateway_max_routes_per_session(500)
            .with_gateway_cursor_safe_tool_names(false)
            .with_deferred_queue_depth(32)
            .with_bridge_queue_depth(64)
            .with_host_queue_depth(8)
            .with_queue_send_timeout_ms(5_000)
            .with_session_ttl_secs(7200)
            .with_job_recovery(JobRecoveryPolicy::Requeue);

        // Gateway sub-config
        let gw = cfg.gateway_config();
        assert_eq!(gw.port, 7000);
        assert_eq!(gw.backend_timeout_ms, 50_000);
        assert_eq!(gw.async_dispatch_timeout_ms, 30_000);
        assert_eq!(gw.wait_terminal_timeout_ms, 120_000);
        assert_eq!(gw.route_ttl_secs, 3600);
        assert_eq!(gw.max_routes_per_session, 500);
        assert!(!gw.cursor_safe_tool_names);

        // Queue sub-config
        let q = cfg.queue_config();
        assert_eq!(q.deferred_queue_depth, 32);
        assert_eq!(q.bridge_queue_depth, 64);
        assert_eq!(q.host_queue_depth, 8);
        assert_eq!(q.send_timeout_ms, 5_000);

        // Session sub-config
        let sess = cfg.session_config();
        assert_eq!(sess.session_ttl_secs, 7200);
        assert_eq!(sess.job_recovery, JobRecoveryPolicy::Requeue);
    }

    /// Issue #764 / backward-compat: `McpHttpConfig::new(port)` produces sensible
    /// defaults for every sub-domain so existing callers that only set `port`
    /// continue to work after the flat-field refactor.
    #[test]
    fn new_port_constructor_defaults() {
        let cfg = McpHttpConfig::new(1234);
        assert_eq!(cfg.port, 1234);
        // Network defaults
        assert_eq!(cfg.host, IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
        assert_eq!(cfg.endpoint_path, "/mcp");
        assert_eq!(cfg.request_timeout_ms, 30_000);
        assert!(!cfg.enable_cors);
        assert_eq!(cfg.spawn_mode, ServerSpawnMode::Ambient);
        // Gateway: disabled by default
        assert_eq!(cfg.gateway_port, 0);
        assert_eq!(cfg.stale_timeout_secs, 30);
        assert_eq!(cfg.heartbeat_secs, 5);
        assert_eq!(cfg.backend_timeout_ms, 120_000);
        assert!(cfg.gateway_cursor_safe_tool_names);
        // Queue: pre-#715 defaults
        assert_eq!(cfg.deferred_queue_depth, 16);
        assert_eq!(cfg.bridge_queue_depth, 16);
        assert_eq!(cfg.host_queue_depth, 0);
        assert_eq!(cfg.queue_send_timeout_ms, 2_000);
        // Session defaults
        assert_eq!(cfg.max_sessions, 100);
        assert_eq!(cfg.session_ttl_secs, 3_600);
        assert!(!cfg.shutdown_on_drop);
        assert_eq!(cfg.job_recovery, JobRecoveryPolicy::Drop);
        // Feature flags
        assert!(!cfg.lazy_actions);
        assert!(cfg.bare_tool_names);
        assert!(cfg.enable_resources);
        assert!(cfg.enable_prompts);
        assert!(!cfg.enable_artefact_resources);
        assert!(cfg.enable_job_notifications);
        assert!(cfg.enable_tool_cache);
    }

    /// Issue #764: `Default::default()` delegates to `new(8765)` so the canonical
    /// default port is preserved.
    #[test]
    fn default_config_port_is_8765() {
        let cfg = McpHttpConfig::default();
        assert_eq!(cfg.port, 8765);
        // bind_addr reflects both host and port correctly.
        assert_eq!(cfg.bind_addr(), "127.0.0.1:8765");
    }
}
