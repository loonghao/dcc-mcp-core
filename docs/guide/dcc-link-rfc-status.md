# DCC-Link RFC status

This page tracks the current implementation status of the DCC-Link RFC and
documents what remains in `dcc-mcp-core`.

## Decision summary

The project adopts `ipckit` as the transport/task substrate for local DCC host
communication, while keeping MCP HTTP as the external client-facing transport.

Design goals from the RFC:

- keep host-side Python runtime dependencies at zero
- make long-running operations first-class (progress + cancellation)
- keep DCC API execution main-thread safe via explicit thread-affinity contracts
- reduce MCP client context pressure with lazy/progressive discovery patterns

## What is already shipped

The following RFC-adjacent work has landed in this repository:

- HTTP progress and cooperative cancellation flow
- `tools/list` pagination + delta notification path
- proactive tool namespacing and SEP-986 validator support
- ResourceLink content for DCC artifact handoff
- initial transport migration slices for ipckit-backed local IPC
- initial `ThreadAffinity` / `HostDispatcher` primitives in process layer

## Active tracking (core)

- #251 — transport migration slices to ipckit-backed local IPC
- #252 — thread-affinity dispatcher contract and reference implementation
- #253 — main-thread pump with time-slice budget and cooperative yield points
- #254 — lazy action schema fast path (discover/describe/call direction)
- #255 — EventStream bridge alignment for MCP progress/cancelled notifications

## What remains in core scope

The high-priority remaining technical work in the RFC chain is:

1. finish thread-affinity + host dispatcher rollout across process/runtime layers
2. add main-thread pump scheduling with bounded time-slice budget
3. decide whether any extra lazy schema surface is still needed beyond pagination + deltas

## Guidance for contributors

- treat this RFC as a **tracking umbrella** rather than a single merge item
- prefer small, mergeable slices per issue with tests
- keep public API compatibility unless an explicit breaking-change path is approved
- new work should link both this umbrella and the concrete child issue
