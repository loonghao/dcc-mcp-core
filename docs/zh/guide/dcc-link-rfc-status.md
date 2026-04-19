# DCC-Link RFC 状态

本文档沉淀了 DCC-Link RFC 的当前状态，帮助维护者快速判断哪些能力已经落地、哪些仍待实现，以及后续实施边界。

## RFC 决策摘要

- 采用 **ipckit** 作为 DCC-Link 的传输与任务基础设施（LocalSocket / TaskManager / EventStream / SharedMemory）。
- 保持 dcc-mcp-core 的核心约束：
  - 主机侧零第三方 Python 运行时依赖（仅本项目 wheel）。
  - 线程亲和（Main / Named / Any）与主线程安全优先。
  - 长任务一等公民（进度、取消、可观测性）。

## 已并入主线

以下能力已在相邻 issue/PR 中落地：

- HTTP 进度通知与协作取消（`notifications/progress` / `notifications/cancelled`）。
- `tools/list` 分页 + 增量更新通知（delta）。
- 主动 skill/tool namespacing。
- DCC 产物的 ResourceLink 内容输出。
- 网关工具名分隔符修复为 `.`，满足 SEP-986。
- 初始 ipckit 传输迁移切片。
- 初始 `ThreadAffinity` / `HostDispatcher` 原语。

说明：RFC 原始 checklist 中与上述能力重叠的项可按"已覆盖"处理。

## 仍需推进的核心子任务

1. **ipckit 传输基座持续收敛**
   - 已有最小接入（见 #251），继续收敛为稳定默认路径。
2. **线程亲和与主线程调度体系**
   - `ThreadAffinity` / `HostDispatcher` 已引入（见 #252）。
   - 仍需主线程时间片 pump（含协作让出）闭环。
3. **lazy actions fast-path**
   - 与分页策略协同，评估是否需要三段式 discover/describe/call 模式。

## 与子 issue 的映射

- #251: transport 适配层切片（ipckit 接入）
- #252: ThreadAffinity / HostDispatcher
- #253: 主线程 pump + 时间片预算
- #254: lazy actions fast-path（与分页策略协同）
- #255: EventStream ↔ MCP 通知桥接（核心能力已被后续实现覆盖）

## 实施建议（给后续 PR）

- 每个 PR 保持"最小可合并切片"，优先可验证的行为变更。
- 文档中引用 RFC 时，始终附加"Reality Check"语境，避免复述过时目标。
- 若能力已在相邻 issue 完成，优先做"对齐与收敛"而非重复实现。
- 新工作应同时链接本 RFC umbrella 和具体的子 issue。
