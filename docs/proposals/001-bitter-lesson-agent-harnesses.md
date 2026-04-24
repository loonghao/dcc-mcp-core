# Proposal 001 — The Bitter Lesson of Agent Harnesses for dcc-mcp-core

- **Status:** Draft — discussion only, no code changes yet
- **Author:** cloud-agent analysis
- **Date:** 2026-04-24
- **Source:** [Gregor Zunic — The Bitter Lesson of Agent Harnesses](https://sotasync.com/reader/2026-04-24-bitter-lesson-agent-harnesses/)
  (translation of [@gregpr07](https://x.com/gregpr07/status/2047358189327520166))
- **Related:** `AGENTS.md` (AI Agent Tool Priority), `skills/README.md`,
  `docs/guide/skills.md`, `examples/skills/maya-geometry/`

> **TL;DR** — Browser Use rebuilt their agent harness from *thousands of lines
> of DOM indexers and click wrappers* down to **~600 lines of thin CDP shims + a
> SKILL.md**. Their lesson: every "helper" you add is an abstraction the model
> has to route around. The model was **already trained on the raw protocol**
> (CDP, DOM, JS) — hand it those primitives and let it write the missing
> functions itself. This proposal audits dcc-mcp-core against that lesson and
> identifies concrete places where we are still *over-wrapping* DCC APIs that
> the LLM already understands.

---

## 1. What the article actually claims

Four concrete claims, each backed by a production anecdote from `browser-harness`:

| # | Claim | Evidence |
|---|-------|----------|
| 1 | **Helpers are abstractions that RL-trained models have to route *around*.** | 数千行 DOM indexer + element extractor + click wrapper → ~600 行纯 CDP 薄封装后,成功率反而更高. |
| 2 | **LLMs are already trained on the raw protocol.** | 模型见过海量 `Page.navigate` / `DOM.querySelector` / `Runtime.evaluate`. You don't need to re-translate it. |
| 3 | **When a helper is *missing*, the agent writes it in-situ.** | `upload_file()` 缺失 → 模型 `grep` 一下,直接用 `DOM.setFileInputFiles` 写了一个。像 fix missing import。 |
| 4 | **When a helper *exists but is wrong*, the agent is stuck.** | 12 MB 上传超过 CDP 10 MB WebSocket 上限 → 模型读了错误信息并自动切换到分块上传。如果 harness 预先"优化"过这条路径,它做不到。 |

The implicit rule: **your harness is a tax**. Every line of helper code is a
hypothesis you've baked in about what the model needs. Every wrong hypothesis
becomes a wall the model has to climb.

---

## 2. How this maps to dcc-mcp-core

dcc-mcp-core is, by design, *the harness* for Maya / Blender / Houdini / Unreal /
Photoshop / ZBrush. We already share Browser Use's goal (expose a DCC to an
agent), and we already made one *aligned* call: **Skills-First + thin tool
registry**. But we've also made several moves that are, in the article's
framing, "replacing a primitive the model already knows with our own wrapper."
Let's audit honestly.

### 2.1 What we already got right

- **`DccLinkFrame` + `IpcChannelAdapter` is a thin transport, not a semantic
  layer.** The frame is `[len][type][seq][msgpack body]`. We route, we don't
  interpret. Good — this is our CDP.
- **Skills are files, not code.** `SKILL.md` + scripts + `tools.yaml` is
  exactly the pattern the article endorses: "a SKILL.md telling the agent how
  to use the tools." The script the model executes is just a Python file with
  `argparse` (see `examples/skills/maya-geometry/scripts/create_sphere.py`).
- **`next-tools.on-failure: [dcc_diagnostics__screenshot, dcc_diagnostics__audit_log]`**
  is literally "when the model gets an error, give it eyes and logs and let it
  figure out the next step." This is the same pattern as the 12 MB chunked
  upload — *don't pre-empt the error handling*, let the model see the raw
  failure and route around it.
- **`execute_python` / `execute_mel` style tools exist in downstream adapters**
  (referenced from `AGENTS.md` and `docs/guide/transport.md`). This is the
  DCC-equivalent of `Runtime.evaluate` — agents *do* reach for it when
  per-operation wrappers fall short.
- **Artefact store + FileRef (#349)** is content-addressed bytes, not a
  "managed render-output object with 18 methods." Agents get the bytes or the
  URI, no wrapping.

### 2.2 Where we are over-wrapping (the honest audit)

These are places where we've inserted abstractions the LLM arguably doesn't
need. Each item is a *discussion point*, not a firm proposal to delete.

#### (A) Fine-grained per-verb Maya tools

`maya-geometry` exposes `create_sphere` / `bevel_edges` / `create_joint` —
each a separate Python script wrapping one `maya.cmds.polyXxx(...)` call
with an `argparse` surface. The model has seen *every `maya.cmds` call that
exists on GitHub*. It does not need us to translate `cmds.polySphere(radius=r,
sx=sdx, sy=sdy)` into a tool with an `input_schema` that has `radius`,
`subdivisionsX`, `subdivisionsY`.

**What the bitter lesson would suggest:** surface **one** thin escape hatch —
`maya__exec(script: str, args: dict)` — that runs arbitrary `maya.cmds` code
on the main thread via `DeferredExecutor`, and make `create_sphere` an
*example in the SKILL.md body*, not an enumerated tool.

**Counter-argument:** `ToolAnnotations` (`destructive_hint`, `idempotent_hint`)
and `input_schema` *do* give the model safety hints it can't derive from a
raw `exec`. Skill authors who want those signals should still be able to
declare them. The right answer is probably **both** — keep the enumerated
tools, but make the raw-exec escape hatch first-class and documented, not
something we quietly discourage.

Action item for a follow-up proposal: **make `execute_python` (or
`dcc__exec`) a *bundled infrastructure skill* alongside `dcc-diagnostics` and
`workflow`.** Every DCC adapter gets it for free. The SKILL.md body tells
the agent: "If no tool matches, write the `maya.cmds` call yourself."

#### (B) Deep `input_schema` trees on thin wrappers

Look at `create_sphere`:

```yaml
input_schema:
  type: object
  properties:
    radius: { type: number, default: 1.0 }
    subdivisionsX: { type: integer, default: 20 }
    subdivisionsY: { type: integer, default: 20 }
```

This is us re-typing [Autodesk's `polySphere` docs](https://help.autodesk.com/view/MAYAUL/2024/ENU/?guid=__CommandsPython_polySphere_html)
into JSON Schema. The model already has those docs in its weights. What
this schema *does* buy us is **validation before the call reaches the DCC
main thread** — a real win for UX (fail fast with a clear error).

**The bitter-lesson read:** validation is fine, but we should be honest that
every `input_schema` we author is **duplication of training data**, and the
payoff is "catch the mistake 50 ms earlier." For rarely-used or rapidly-
evolving DCC APIs, we should not block the agent until we've authored a
schema — let the DCC itself be the validator.

**Concrete change:** add an explicit section to `skills/README.md` called
*"When *not* to author an `input_schema`"* — if the underlying DCC command
is well-documented, volatile, or rarely hit, skip the schema and let the
error bubble up. Pair the raw-exec hatch (A) with this guidance.

#### (C) The `DccBridge` WebSocket JSON-RPC layer

`DccBridge` wraps non-Python DCCs (Photoshop, ZBrush, Unity) in WebSocket
JSON-RPC 2.0. This is *exactly* the pattern the article warns about — we
are inventing a semantic layer on top of what is already a protocol the
vendor ships.

- Photoshop already has **UXP + CEP messaging**. Agents have seen UXP tutorials.
- Unity has `EditorWindow.SendMessage` + `UnityEditor.EditorApplication.update`.
- ZBrush has ZScript.

Today, a `DccBridge` skill tool maps 1:1 to a JSON-RPC method the bridge
exposes. That's **two translations**: agent → our tool schema → our JSON-RPC
→ vendor API. The bitter lesson says to collapse that to **one**: expose the
vendor's native messaging channel as a single tool (e.g.
`photoshop__uxp_exec(code)`) and let the agent write the UXP directly.

**Recommendation:** keep `DccBridge` as infrastructure (we *do* need to carry
the bytes), but add a first-class `uxp_exec` / `zscript_exec` style tool as
the *preferred* entry point, and demote fine-grained per-verb bridge tools to
examples in the SKILL.md body.

#### (D) `ToolPipeline` middleware chain

We kept `LoggingMiddleware`, `ValidatorMiddleware`, `AuditMiddleware` even
after removing the legacy `ActionManager` in v0.12+. Each middleware is a
helper in the article's sense — code we wrote *for the model's benefit* that
the model will have to route around if it misbehaves.

`AuditMiddleware` survives the bitter-lesson test (it's for *humans*, not the
model — compliance, debugging, trust boundary). `ValidatorMiddleware` is
50/50 (see B). `LoggingMiddleware` and other pipeline additions should be
justified *against human operators*, not "to help the model." If the only
answer is "the model uses this," the bitter-lesson move is to delete it and
let the model read the raw DCC traceback.

#### (E) Too many built-in MCP tools on `tools/list`

`McpHttpServer` today registers a non-trivial number of built-in tools even
before any skill loads: `jobs.get_status`, `jobs.cleanup`, `workflows.run`,
`workflows.get_status`, `workflows.cancel`, `workflows.lookup`,
`search_skills`, `load_skill`, `activate_tool_group`, plus whatever
`lazy_actions` decides to surface. Each one is token budget on every
`tools/list`.

The article's 600-line harness exposes **one or two entry points**:
`navigate(url)` and `exec(js)`. Everything else is discoverable from there.

Our current model is defensible (progressive exposure via `__skill__<name>`
stubs keeps the list short, and `bare_tool_names` collapses
`maya-scripting.execute_python` → `execute_python`). But we should
periodically ask: *does this built-in tool carry its weight on `tools/list`,
or could it be a sub-command of `workflow__run` / `diagnostics__*`?*

Low-hanging fruit: consolidate `jobs.get_status` + `jobs.cleanup` +
`workflows.get_status` + `workflows.cancel` behind a single `jobs__*` prefix
and make it a single bundled skill, so an agent that doesn't care about job
lifecycle never sees those 4 slots.

#### (F) The `thread_affinity="main"` and `DeferredExecutor` dance

This is the place where I think the bitter lesson **does not apply** and I
want to flag it explicitly to preempt over-correction. DCC main-thread
affinity is not a model-facing abstraction — it's an OS-level invariant
(Maya will crash if you call `cmds.*` from a non-main thread). Removing this
wrapper would not "let the model figure it out"; it would cause non-
deterministic crashes that the model cannot recover from even with a
traceback, because the process is gone.

Keep `DeferredExecutor`. Keep `@chunked_job`. These are *load-bearing*
abstractions, not "helpers for the model."

---

## 3. The "bitter-lesson gradient" — a heuristic for future design reviews

I propose adding this to `AGENTS.md` → *Do and Don't*:

> **Before adding a new helper / wrapper / middleware, ask:**
>
> 1. **Does the raw primitive crash the process?** (DCC main thread, memory
>    safety, kernel handles.) → Keep the wrapper. Non-negotiable.
> 2. **Is the wrapper for *human* operators?** (audit log, metrics, SBOM.) →
>    Keep it, but document *who the reader is*, not "to help the AI."
> 3. **Does the wrapper translate a protocol the LLM already knows?**
>    (`maya.cmds`, UXP, bpy, hou, JS, CDP, HTTP, SQL.) → **Default to
>    NOT adding it.** Expose the raw escape hatch first. Add the wrapper
>    only when you have evidence (telemetry, failed eval) that the agent
>    cannot route the primitive.
> 4. **Does the wrapper give safety hints the raw call can't?**
>    (`ToolAnnotations.destructive_hint`, `read_only_hint`.) → Keep it,
>    but keep it *thin* — one JSON Schema, not a whole middleware stack.

This is the same policy that made `DccLinkFrame`, `FileRef`, and
`success_result` good designs. Let's make it explicit.

---

## 4. Concrete follow-up proposals (each to be a separate PR)

None of this is in-scope for this proposal — each item below deserves its
own discussion and review. This proposal is a **design-review checklist**,
not an implementation plan.

1. **`dcc-exec` bundled infrastructure skill** — universal
   `maya__exec`, `blender__exec`, `houdini__exec`, `unreal__exec`,
   `photoshop__uxp_exec` etc. Thin wrapper over each DCC's native scripting
   primitive, routed through `DeferredExecutor` where applicable. SKILL.md
   body includes "cookbook" examples for the top 20 verbs per DCC, so the
   agent has few-shot demos without us committing to a schema.
2. **"When not to author an `input_schema`" section in `skills/README.md`** —
   explicit guidance that for rarely-used or volatile DCC commands, skipping
   the schema is a valid choice. Pair with telemetry: track which schemas
   catch real agent mistakes vs which are pure training-data duplication.
3. **Built-in tool audit** — review every tool in `build_core_tools_inner`
   and `build_lazy_action_tools` (`crates/dcc-mcp-http/src/handler.rs`)
   against the 4-question heuristic. Consolidate `jobs.*` and
   `workflows.*` behind a single infrastructure skill so the server-level
   `tools/list` starts closer to 3 entries than 10+.
4. **Bitter-lesson gradient added to `AGENTS.md`** — the 4-question rubric
   above, as a Do/Don't entry under *Writing Tool Descriptions* → *Writing
   Tool **Wrappers***.
5. **Docs page `docs/guide/harness-philosophy.md`** — translate the bitter
   lesson into a stable design doc the project can reference in future
   design reviews. Link from `AGENTS.md` → *Decision Tree*.

---

## 5. What this proposal is **not**

- **Not** a proposal to delete `input_schema`, `ToolAnnotations`,
  `DeferredExecutor`, or `DccBridge`. Each earns its keep today.
- **Not** a proposal to remove fine-grained skills. `maya-geometry` is a
  good *example* skill; it should continue to exist. The question is
  whether *every* adapter should have to author 60 of them, or whether a
  `dcc-exec` + 5 `ToolAnnotations`-annotated highlights is enough for most
  cases.
- **Not** applicable to load-bearing infrastructure (thread safety, IPC
  framing, artefact hashing, job persistence). Those wrappers exist because
  the *OS* or the *protocol* demands them, not because we think the model
  needs help.

---

## 6. Call to action

- Reviewers: please identify specific places in your corner of the codebase
  where a helper was added "to help the model" without telemetry evidence.
  File each one as a comment on this proposal.
- Once we converge on the heuristic (Section 3), I'll open a PR to land it
  in `AGENTS.md` — that's the smallest possible first step.
- Items in Section 4 each become separate proposals / PRs, not rolled
  into this one.

## References

- [Gregor Zunic — The Bitter Lesson of Agent Harnesses](https://x.com/gregpr07/status/2047358189327520166)
- [browser-harness](https://github.com/browser-use/browser-harness)
- [Rich Sutton — The Bitter Lesson](http://www.incompleteideas.net/IncIdeas/BitterLesson.html)
  (the original 2019 essay this argument is built on)
- Internal: `AGENTS.md` — *AI Agent Tool Priority* + *Writing Tool Descriptions*
- Internal: `skills/README.md` — *Skill Layering*
- Internal: `docs/guide/skills.md` — *next-tools* and search ranking
