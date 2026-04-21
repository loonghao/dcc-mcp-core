# Workflow Orchestration Gap Analysis

> **Status**: Proposal — each section below is a GitHub issue draft.
> **Context**: Feasibility analysis for end-to-end pipelines such as
> _"import yesterday's vendor-submitted Maya scenes from SFTP, run the QC
> skill on each, and export passing ones as FBX into Unreal"_.
>
> **Reader**: repo maintainers — use this file to cut the issues below
> and to patch the three existing issues flagged in the **Amendments**
> section. The cloud agent environment's `gh` CLI is read-only, so this
> file is the vehicle rather than direct issue creation.

---

## 1. Landscape — What we already have

| Layer | Primitive | Status |
|---|---|---|
| Single tool execution | `ToolRegistry` / `ToolDispatcher` / `ToolPipeline` | Shipped |
| Long-running single tool | `JobManager` (#316), async dispatch (#318), polling (#319), SSE (#320, #326), persistence (#328), cancellation (#329), main-thread safety (#332) | Partly shipped (#316 in), rest scoped |
| Cross-step context inside one server | `ActionChain` (`crates/dcc-mcp-actions/src/chain.rs`) | Shipped but **not exposed as a built-in MCP tool or a SKILL.md concept** |
| Cross-DCC discovery | Gateway (`McpHttpConfig.gateway_port`), `DccGatewayElection`, `FileRegistry` | Shipped |
| Cross-DCC sync call | Gateway request routing | Shipped |
| Cross-DCC async call | #321 (passthrough), #322 (routing cache) | Scoped |
| User confirmation | `elicitation/create` (2025-06-18) | Shipped in handler |
| Skill chaining hints | SKILL.md `next-tools: on-success/on-failure` | Metadata only, not enforced (#342) |
| File in/out | Nothing. Tools accept/return JSON only. | **Gap** |
| Credentials / external systems | Nothing. No SFTP, S3, PDM, render-farm abstraction. | **Gap — out of scope for core, belongs in skills** |

## 2. Canonical scenario breakdown

_"Import yesterday's vendor submissions from `sftp://vendor-drop/2026-04-21/`
into Maya, run the QC skill on each, export passing ones as FBX, drop them
into Unreal's `Content/Incoming/`."_

Decomposition against current primitives:

| Step | What it needs | Where it lives | Gap? |
|---|---|---|---|
| 1. List remote files filtered by date | A `vendor-intake` **skill** with SFTP/S3 dependency declared via `external_deps` | User-authored skill (not core) | ✅ core already supports `external_deps` |
| 2. For each file: open Maya, import | Per-file sub-job, must be serialised on Maya main thread | `JobManager` + `DeferredExecutor` (#332) | ✅ if #318/#332 land |
| 3. Run QC skill, branch on pass/fail | ToolPipeline + SKILL.md `next-tools: on-failure` | SKILL.md + #342 | ⚠️ #342 only wires hints, not enforcement |
| 4. Export FBX to staging path | A `maya-export` tool | User skill | ✅ |
| 5. Hand FBX over to Unreal MCP | Cross-DCC call (maya → unreal) from within one job | Gateway + cross-DCC call | ⚠️ works for sync, but the outer job needs to wait on another DCC's async job |
| 6. Aggregate per-file results, report to client | N steps in one outer "workflow-run", progress, partial-fail semantics | **No primitive** | ❌ `ActionChain` exists in Rust but not exposed as MCP tool / Job and has no parallel, fan-out, or sub-job semantics |
| 7. Resume on server restart | Persistence of workflow state, not just leaf jobs | #328 covers leaf jobs only | ❌ |
| 8. Human approval before write steps | `elicitation/create` per step | Already in 2025-06-18 handler | ✅ available but no idiomatic pattern in `ActionChain` |
| 9. Audit trail for compliance | `AuditLog` / `AuditMiddleware` | Shipped | ✅ but workflow-level spans missing |
| 10. Artefact handoff between steps | Pass "file reference" between tools without inlining bytes | **No primitive** | ❌ |

So feasibility for your example workflow today: **yes, but the glue is
Python that the integrator writes**; the core library does not yet model
"a multi-step DCC pipeline" as a first-class Job. The six items below
close that gap.

## 3. Design principles for the workflow layer

1. **Leaf-tool reusability.** A workflow is N reused tools, not a new
   handler type. No SKILL.md change is required for leaf tools.
2. **Workflow = special skill + special job.** A workflow is authored as
   a `SKILL.md` with a `workflow:` frontmatter section and runs as an
   async `Job` that itself dispatches child Jobs.
3. **No new runtime dependencies.** Persistence reuses #328's sqlite
   feature flag. Transport reuses gateway + SSE. Zero Python deps.
4. **Main-thread affinity preserved.** Per-DCC leaf jobs go through
   `DeferredExecutor`; workflow orchestration runs on Tokio, never on
   the DCC thread.
5. **Human-in-the-loop is a step kind, not a hack.** `elicitation/create`
   is wrapped as an explicit `approve` step.
6. **Spec-aligned signalling.** Reuse `notifications/progress` for the
   outer workflow token; emit `$/dcc.jobUpdated` for leaf jobs and a new
   `$/dcc.workflowUpdated` for workflow-level transitions.

## 4. New issues (to be cut)

### Issue A — `feat(core): first-class Workflow primitive (WorkflowSpec + WorkflowJob)`

**Labels**: `enhancement`, `long-tasks`, `mcp`

**Summary**
Promote `ActionChain` to a spec-driven, persistable, cancellable,
partial-failure-aware `WorkflowJob` that runs as an async MCP tool. This
is the smallest building block for "do N tool calls as one pipeline with
one progress token and one job id".

**Motivation**
Real DCC pipelines (vendor intake → QC → export → handoff) are always
N-step sequences where some steps fan out over a list and any step may
fail. Today the integrator must reimplement job tracking, progress,
cancellation, and partial failure on top of `ActionChain`. That glue
belongs in core.

**Proposed design**

- New crate module `crates/dcc-mcp-actions/src/workflow/` (or new crate
  `dcc-mcp-workflow` if tests show circular deps with `chain.rs`).
- `WorkflowSpec` — declarative DAG, nodes are `Step { id, kind, ... }`:
  - `kind: tool` — call one MCP tool on the local registry.
  - `kind: tool_remote` — call one MCP tool on another DCC via gateway.
  - `kind: foreach` — iterate over a JSONPath expression in the context,
    child subgraph runs per item with its own step results.
  - `kind: parallel` — run N children concurrently; respects per-DCC
    main-thread affinity (parallel across DCCs, serial within one DCC).
  - `kind: approve` — issue `elicitation/create`, block until accept /
    decline / cancel, surface as a step result.
  - `kind: branch` — `on: $.last_step.passed` → `then: [...]` /
    `else: [...]`.
- `WorkflowJob` — an async `Job` (in the existing `JobManager`) whose
  status is computed from child jobs; exposes aggregated progress
  (`completed_steps / total_steps`) plus `current_step_id`.
- `structuredContent` shape for the outer `CallToolResult`:

  ```json
  {
    "workflow_id": "<uuid>",
    "job_id": "<uuid>",
    "status": "pending",
    "steps": [
      {"id": "intake", "status": "pending"},
      {"id": "qc",     "status": "pending"},
      {"id": "export", "status": "pending"}
    ]
  }
  ```

- Built-in tool: `workflows.run` (SEP-986 compliant) — input is an inline
  `WorkflowSpec` **or** a `workflow_name` resolved from a loaded skill's
  `workflow:` frontmatter section. Returns `workflow_id + job_id`.
- Built-in tool: `workflows.get_status` — polls aggregated state
  including each step's child job snapshot.
- Built-in tool: `workflows.cancel` — cancels the outer job, which
  cascades to child jobs via their `CancellationToken`.

**Persistence**
Reuses #328's sqlite feature flag. Adds two tables: `workflows` and
`workflow_steps`. On recovery, workflows in `Pending`/`Running` are
re-walked: finished steps stay finished, the first unfinished step is
set to `Interrupted` and the workflow status to `Interrupted` (new, same
as #328's rationale). Clients choose whether to resume (`workflows.resume`
— separate built-in tool).

**SKILL.md extension (opt-in frontmatter block)**
```yaml
workflows:
  - name: vendor_intake
    description: "Import vendor Maya files, QC, export FBX, push to Unreal."
    inputs:
      date: { type: string, format: date }
    steps:
      - id: list
        tool: vendor_intake__list_sftp
        args: { date: "{{inputs.date}}" }
      - id: per_file
        kind: foreach
        items: "$.list.files"
        as: file
        steps:
          - id: import
            tool: maya__import_scene
            args: { path: "{{file.path}}" }
          - id: qc
            tool: maya_qc__run_all
          - id: gate
            kind: branch
            on: "$.qc.passed"
            then:
              - id: export
                tool: maya__export_fbx
                args: { out_dir: "/mnt/staging/fbx/" }
              - id: handoff
                kind: tool_remote
                dcc: unreal
                tool: unreal__ingest_fbx
                args: { src: "{{export.out_path}}" }
```

**Acceptance criteria**
- `workflows.run` / `workflows.get_status` / `workflows.cancel` all listed
  in `tools/list`, all pass `validate_tool_name`.
- Running the example spec above against a mock registry with 3 files
  completes with 3 successful `handoff` rows in the final structured
  content.
- Cancelling the outer job cascades: all in-flight child jobs observe
  their `cancel_token` within one cooperative checkpoint (coordinates
  with #329).
- Main-thread affinity preserved: each DCC's children serialise through
  `DeferredExecutor`; parallel across DCCs confirmed by timing test.
- Persistence: kill the server mid-run, restart with same
  `job_db_path`, `workflows.get_status` reports `Interrupted` with
  exact `current_step_id`. `workflows.resume` continues from there.
- Spec: emits `notifications/progress` (0..total_steps) against the
  outer `progressToken`; emits `notifications/$/dcc.workflowUpdated`
  on each step transition. Leaf job events from #326 remain unchanged.
- SKILL.md with `workflows:` frontmatter is parsed into the catalog;
  `search_skills` surfaces workflows with `kind: workflow` in the hit
  payload.

**Non-goals**
- No visual editor.
- No scheduling / cron / triggers (see Issue E).
- No retries with backoff (see Issue F).

---

### Issue B — `feat(core): artefact handoff via FileRef resources`

**Labels**: `enhancement`, `mcp`, `long-tasks`

**Summary**
Introduce a `FileRef` value object and wire it into `ToolResult` +
`workflow` context so that steps can pass "a file" without inlining
bytes. Expose it via the MCP Resources primitive so MCP clients can
also read the artefact.

**Motivation**
Pipelines move files between tools (imported scene → QC report → FBX
→ staged .uasset). Today the only way to pass this between steps is an
absolute path string in the context, which:
- Breaks across machines (gateway → remote DCC).
- Has no MIME / checksum / size → clients can't validate.
- Is not MCP Resources — clients can't `resources/read` it.

**Proposed design**
- New `FileRef` struct:
  ```rust
  pub struct FileRef {
      pub uri: String,        // e.g. "artefact://<workflow_id>/<step_id>/scene.mb"
      pub local_path: Option<PathBuf>,
      pub remote: Option<RemoteRef>, // sftp/s3/http(s)
      pub mime_type: Option<String>,
      pub size_bytes: Option<u64>,
      pub sha256: Option<String>,
      pub expires_at: Option<DateTime<Utc>>,
  }
  ```
- Helper `success_result_with_files(msg, files=[...])` that stamps
  `_meta.dcc.files = [FileRef...]` on the `CallToolResult`.
- `WorkflowJob` (Issue A) propagates `FileRef`s from one step's
  structured content into the next step's interpolation context.
- Opt-in MCP Resources integration: workflow produces resources under
  `artefact://<workflow_id>/…`; the server's `resources/list` surfaces
  active artefacts for the lifetime of the workflow job. `resources:
  { subscribe: true, listChanged: true }` capability advertised only
  when the feature is enabled via `McpHttpConfig.enable_artefact_resources`.
- Cleanup: on workflow terminal state, `FileRef`s marked `ephemeral`
  are deleted; others remain but stop being advertised.

**Acceptance criteria**
- `FileRef` round-trips through `ToolResult` → chain context → next
  step's args via `{{...}}` interpolation.
- Gateway rewrites `local_path` to a `remote` pointer when crossing
  DCC boundaries (backed by the existing SSE/session).
- `resources/list` lists active artefacts; `resources/read` returns
  them via a length-prefixed stream; 404 after workflow terminal.
- Zero new Python deps.

---

### Issue C — `feat(http): Resources primitive for live DCC state`

**Labels**: `enhancement`, `mcp`

**Summary**
Advertise `resources: { subscribe, listChanged }` in server capabilities
and expose `resources/list`, `resources/read`, `resources/subscribe`.
Initial producers:
- `scene://current` — JSON summary from `SceneInfo`.
- `capture://current_window` — PNG snapshot via `Capturer.new_window_auto`.
- `audit://recent?limit=N` — `AuditLog` tail.
- `artefact://…` — from Issue B.

**Motivation**
Tools are model-controlled, prompts are user-controlled, resources are
**application-attached**. In Cursor / Claude Desktop a user can drag
`scene://current` into the conversation; in an agent orchestration it
is the natural place for "give me the current scene summary" without
burning a tool call.

**Acceptance criteria**
- `initialize` advertises resources capability when
  `McpHttpConfig.enable_resources = true`.
- All four producers return valid MCP 2025-03-26 resources.
- Subscribe emits `notifications/resources/updated` on scene change
  (hooked into existing event bus).
- Documented that resources are additive to tools, not a replacement.

---

### Issue D — `feat(skills): prompts primitive derived from SKILL.md examples + workflows`

**Labels**: `enhancement`, `mcp`

**Summary** _(expansion of the conversation from earlier — keep as its
own issue)_
Flip `ServerCapabilities.prompts` to `Some(...)`. Auto-derive prompts
from each loaded skill's `examples` and from each workflow declared in
a skill's `workflows:` block (Issue A).

**Motivation**
Workflows are the exact entry points artists want in a `/` menu
(`/maya.vendor_intake`, `/unreal.ingest_batch`). Prompts give humans a
parameter form; tools give agents an API. Workflows serve both.

**Acceptance criteria** _(as in the earlier proposal, plus)_
- Each `WorkflowSpec` with `inputs:` produces one prompt whose arguments
  mirror `inputs`.
- Invoking the prompt dispatches `workflows.run` with the collected args.

---

### Issue E — `feat(core): scheduled workflow triggers (cron + webhook)`

**Labels**: `enhancement`, `long-tasks`

**Summary**
Let workflows run on a schedule or on a file/webhook event without a
human tool call. "Every morning at 08:00, run vendor_intake for
yesterday" is the classic DCC case.

**Proposed design**
- New crate `dcc-mcp-scheduler` (optional via feature flag
  `scheduler`). Uses `tokio-cron-scheduler` or a minimal custom tick.
- Built-in tools:
  - `schedules.create` — cron expression + `workflow_name` + inputs.
  - `schedules.list` / `schedules.delete` / `schedules.pause`.
- File triggers use existing `SkillWatcher` / notify patterns but watch
  a "inbox" directory instead of skills. Debounce 1–5 s.
- Webhook triggers land on an extra HTTP endpoint
  (`/triggers/<id>`, POST) issued per schedule.
- Persistence shared with Issue A's sqlite.

**Acceptance criteria**
- Cron trigger fires `workflows.run` at the declared time with a fresh
  correlation id.
- Inbox trigger debounces and dedups by file stable-path.
- All triggers produce workflow jobs visible in `workflows.list` with
  a `trigger: cron|inbox|webhook|manual` attribution.
- Disabled by default (feature flag); zero Python deps.

---

### Issue F — `feat(core): step-level retry, timeout, and idempotency keys`

**Labels**: `enhancement`, `long-tasks`, `concurrency`

**Summary**
Production pipelines need to survive transient DCC hiccups without
failing the whole workflow. Give each workflow step:
- `retry: { max, backoff: { initial_ms, multiplier, jitter }, on: [errors] }`
- `timeout_ms: u64` — soft deadline that triggers cancellation + retry.
- `idempotency_key: "{{item.path}}"` — skipped if a prior successful
  step with the same key exists in the persistence layer.

**Motivation**
A crashed Maya import on file 37/250 should not force rerunning 0..36.
Idempotency keys make "resume after crash" and "retry flaky step"
safe.

**Acceptance criteria**
- Retries honour cancellation (abort mid-retry on `workflows.cancel`).
- Timeout uses the existing `cancel_token` mechanism from #329.
- Idempotency key uniqueness enforced per `workflow_name + step_id`.
- Metrics: retry count per step exported via #331.

---

### Issue G — `feat(skills): capability declaration + typed workspace path handshake`

**Labels**: `enhancement`, `mcp`, `integration`

**Summary**
Workflows need to know what a DCC can do before issuing a step. Today
`DccCapabilities` exists but is only surface-level. Add:

- `SkillRequirements { requires_capabilities: [String], requires_dcc_version: VersionConstraint }`
  in SKILL.md frontmatter — workflow dispatcher skips steps the DCC
  can't satisfy (branch to `else:` or fail fast).
- `WorkspaceRoot` capability — each DCC reports a typed workspace root
  set (project, staging, render-output). Workflows reference them as
  `{{workspace.staging}}/fbx/…` instead of hardcoding platform paths.

**Motivation**
"Same workflow, different studio layout" — studios plug different
workspace roots via env vars or a config file; core resolves them.

**Acceptance criteria**
- New MCP tool `workspace.describe` returns `{ roots: {project, staging,
  renders, ...}, platform, user }`.
- `WorkflowJob` interpolates `{{workspace.<name>}}` at step time, not
  at spec load time.
- `requires_capabilities` is validated at workflow-load time, producing
  a human-readable error before any work starts.

---

## 5. Amendments to existing issues

> These are direct edits to issue bodies (no comments). Each edit keeps
> the original scope intact but adds cross-links and constraints so
> workflows can build on them cleanly.

### Patch for #318 (async dispatch)

Add a new section before "Non-Goals":

```md
## Nesting Under a Workflow

When the async dispatch is the result of a `workflow` step dispatch,
the caller MUST pass `_meta.dcc.parentJobId` in the original
`tools/call`. The newly created child `Job` records `parent_job_id`,
and its status transitions MUST propagate to the parent `WorkflowJob`
(see Issue A). Cancellation on the parent cascades to child jobs via
their shared `CancellationToken`.
```

Acceptance additions:
- [ ] `_meta.dcc.parentJobId` is recorded on the child `Job` and
      surfaced in `jobs.get_status` output.
- [ ] Cancelling a parent `WorkflowJob` cancels every descendant
      child job within one cooperative checkpoint.

### Patch for #326 (notifications)

Add under "Proposed Change":

```md
### C) Workflow lifecycle events — `notifications/$/dcc.workflowUpdated`

Workflow transitions (step enter / step terminal / workflow terminal)
emit a separate channel to avoid overloading the leaf-job channel:

{
  "method": "notifications/$/dcc.workflowUpdated",
  "params": {
    "workflow_id": "...",
    "job_id": "...",
    "status": "running|completed|failed|cancelled|interrupted",
    "current_step_id": "qc",
    "progress": {"completed_steps": 3, "total_steps": 8}
  }
}

Leaf-level `$/dcc.jobUpdated` events continue unchanged; clients that
care about the outer pipeline subscribe to the workflow channel.
```

Acceptance additions:
- [ ] Workflow channel emits once per step transition and once per
      terminal.
- [ ] Leaf-level channel is not backfilled with workflow-level info.

### Patch for #328 (job persistence)

Add a "Workflow storage" section:

```md
## Workflow Storage

When Issue A (first-class workflows) lands, the same
`job-persist-sqlite` feature owns two additional tables: `workflows`
and `workflow_steps`. A workflow row references its own `job_id` in
the `jobs` table; step rows reference both the parent `workflow_id`
and their child `job_id`. Recovery policy for workflows mirrors the
one for jobs: interrupted mid-run → `Interrupted` + pointer to the
first unfinished step; `workflows.resume` is the explicit opt-in.
```

Acceptance additions:
- [ ] `workflows` and `workflow_steps` tables exist when feature is on;
      do not exist in default build.
- [ ] `workflows.get_status` after a restart returns `Interrupted`
      with a populated `current_step_id`.

---

## 6. Not proposed (intentionally)

- **A full DAG engine (Airflow-like).** Out of scope — workflows here
  are linear with `foreach` / `branch` / `parallel`. Studios already
  operate render farms for heavy distributed work; core is the
  orchestrator that talks to DCCs, not a replacement for the farm.
- **A UI.** Prompts (Issue D) + Resources (Issue C) cover the human
  surface; rendering belongs in the client (Claude Desktop, Cursor,
  custom WebView via `WebViewAdapter`).
- **Custom DSL for step expressions.** Use `jsonpath_rust` for
  selectors and `{{...}}` minijinja for interpolation, both
  zero-Python-dep, both standard.
- **Multi-user RBAC.** Reuse `SandboxPolicy` + `SkillScope`; no new
  identity system.

## 7. Implementation order (technical risk, not calendar)

1. Issue A (workflow primitive) — touches `dcc-mcp-actions`,
   `dcc-mcp-http`, `dcc-mcp-models`. High blast radius, all new code.
2. Patches to #318 / #326 / #328 — small, direct.
3. Issue F (retry/timeout/idempotency) — depends on A + #328.
4. Issue B (FileRef) — depends on A; small, orthogonal to others.
5. Issue G (capabilities + workspace roots) — depends on A.
6. Issue C (Resources) — independent of A; can happen first if a
   contributor prefers.
7. Issue D (prompts derived from workflows) — depends on A + Issue C
   pattern.
8. Issue E (schedulers) — last; requires A + F to be stable.

---

## 8. Answer to the original question

> "Can the vendor → Maya → QC → FBX → Unreal example be expressed
> through our DCC MCP today?"

**Partially.** The pieces exist (skills, gateway, elicitation, async
jobs on the roadmap). What is missing is the **workflow layer** —
without it the integrator writes ad-hoc Python glue that duplicates
`JobManager`, progress, cancellation, resume, and partial-failure
handling. With Issue A + F + B landed, the example becomes a single
`SKILL.md` authored by a TD, invoked via `/vendor_intake date:2026-04-21`
in Cursor, producing one outer job id the artist can cancel, resume, or
inspect without writing any Python.
