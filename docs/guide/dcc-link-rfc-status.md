# DCC-Link RFC status (Issue #249)

Issue link: <https://github.com/loonghao/dcc-mcp-core/issues/249>

This page records the **current implementation reality** for the DCC-Link RFC
track so feature PRs can reference one stable source of truth.

## Original decision

- Adopt **ipckit** as the transport/task substrate for DCC-Link.
- Keep host-side Python dependency model at zero third-party runtime deps.
- Make thread-affinity and long-running task behavior first-class concerns.

## What is already shipped

The following RFC-adjacent work has landed in this repository:

- HTTP progress and cooperative cancellation flow.
- `tools/list` pagination + delta notification path.
- proactive tool namespacing and SEP-986 validator support.
- ResourceLink content for DCC artifact handoff.
- initial transport migration slices for ipckit-backed local IPC.
- initial `ThreadAffinity` / `HostDispatcher` primitives in process layer.

## Active tracking (core)

- #251 — transport migration slices to ipckit-backed local IPC
- #252 — thread-affinity dispatcher contract and reference implementation
- #253 — main-thread pump with time-slice budget and cooperative yield points
- #254 — lazy action schema fast path (discover/describe/call direction)
- #255 — EventStream bridge alignment for MCP progress/cancelled notifications

## Practical status notes

- The RFC remains a **tracking umbrella** rather than a single merge item.
- Individual capabilities are delivered through focused child issues/PRs.
- New work should link both this umbrella and the concrete child issue.
