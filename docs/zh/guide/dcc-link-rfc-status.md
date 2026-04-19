# DCC-Link RFC 状态（#250）

本文档沉淀了 [#250](https://github.com/loonghao/dcc-mcp-core/issues/250) 的当前状态，帮助维护者快速判断哪些能力已经落地、哪些仍待实现，以及后续实现边界。

## RFC 决策摘要

- 采用 **ipckit** 作为 DCC-Link 的传输与任务基础设施（LocalSocket / TaskManager / EventStream / SharedMemory）。
- 保持 dcc-mcp-core 的核心约束：
  - 主机侧零第三方 Python 运行时依赖（仅本项目 wheel）。
  - 线程亲和（Main / Named / Any）与主线程安全优先。
  - 长任务一等公民（进度、取消、可观测性）。

## 与已合并工作的关系

RFC 编写后，以下能力已通过相邻 PR 落地：

- `#256`：MCP 进度通知与协作取消（`notifications/progress` / `notifications/cancelled`）。
- `#257`：`tools/list` 分页 + 增量更新通知（delta）。
- `#258`：主动 skill/tool namespacing。
- `#259`：DCC 产物的 ResourceLink 内容输出。
- `#261`：网关工具名分隔符修复为 `.`，满足 SEP-986（去除 `/` 形式）。

结论：RFC 中“事件桥接到 MCP 进度通知”已基本被后续实现覆盖，后续工作应避免重复建设。

## 当前重点（Remaining Scope）

在现有代码基线下，#250 关联工作建议聚焦于：

1. **ipckit 传输基座持续收敛**
   - 已有最小接入（见 #251），继续收敛为稳定默认路径。
2. **线程亲和与主线程调度体系**
   - `ThreadAffinity` / `HostDispatcher` 已引入（见 #252）。
   - 仍需主线程时间片 pump（含协作让出）闭环。
3. **文档与运行手册同步**
   - 明确“RFC 原始清单 vs 已落地现实”的差异，避免实施偏航。

## 与懒加载工具面的关系

RFC 原始草案里有三段式 `discover → describe → call` 方向。当前主线已经采用：

- 分页 `tools/list`
- 增量工具变更通知

因此三段式模式应视为**可选优化路径**，而不是当前默认交互模型。

## 关联 issue（追踪）

- #251: transport 基座切片（ipckit 接入）
- #252: ThreadAffinity / HostDispatcher
- #253: 主线程 pump + 时间片预算
- #254: lazy actions fast-path（与分页策略协同）
- #255: EventStream ↔ MCP 通知桥接（核心能力已被后续实现覆盖）
- #260: SEP-986 命名校验器
- #261: 网关分隔符规范修复

## 实施建议（给后续 PR）

- 每个 PR 保持“最小可合并切片”，优先可验证的行为变更。
- 文档中引用 RFC 时，始终附加“Reality Check”语境，避免复述过时目标。
- 若能力已在相邻 issue 完成，优先做“对齐与收敛”而非重复实现。
