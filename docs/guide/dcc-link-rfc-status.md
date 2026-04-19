# DCC-Link RFC status (issue #250)

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

## Current status snapshot

Several RFC-adjacent items are already merged:

- progress/cancellation notifications over MCP HTTP are implemented
- paginated `tools/list` and delta updates are implemented
- SEP-986 name validation and gateway separator compatibility fixes are implemented

Recent transport migration slice (issue #251):

- local IPC path in `dcc-mcp-transport` now routes through ipckit async local sockets
- public transport API remains stable (`TransportAddress`, `IpcListener`, `connect_ipc`)

## What remains in core scope

The high-priority remaining technical work in the RFC chain is:

1. finish thread-affinity + host dispatcher rollout across process/runtime layers
2. add main-thread pump scheduling with bounded time-slice budget
3. decide whether any extra lazy schema surface is still needed beyond pagination + deltas

## Mapping to tracking issues

- #251: transport migration slice to ipckit local IPC
- #252: thread-affinity dispatcher primitives
- #253: main-thread pump and cooperative scheduling
- #254: lazy action exposure strategy
- #255: progress/cancel bridge (already covered by merged HTTP work)

## Guidance for contributors

- treat this RFC as an architecture tracking umbrella
- prefer small, mergeable slices per issue with tests
- keep public API compatibility unless an explicit breaking-change path is approved
