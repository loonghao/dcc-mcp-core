# DCC-Link RFC 状态（Issue #249）

> 对应 GitHub issue：[#249](https://github.com/loonghao/dcc-mcp-core/issues/249)
>
> 本页面用于沉淀 RFC 的当前状态，区分：
> - 已并入主线的能力
> - 仍待实现的核心子任务
> - 上游 `ipckit` 依赖项

## RFC 目标（摘要）

RFC #249 目标是在 DCC 生态中统一采用 `ipckit` 作为传输/任务基座，以支持：

- 长任务（进度 + 取消）
- 线程亲和（`Main` / `Named` / `Any`）
- 主线程安全调度
- 低上下文成本的工具发现

## 已并入主线（截至当前分支）

以下能力已在相邻 issue/PR 中落地：

- Progress/Cancel 通知桥接
- `tools/list` 分页 + 增量通知
- 资源链接（ResourceLink）输出
- SEP-986 命名约束与网关分隔符修复

说明：RFC 原始 checklist 中与上述能力重叠的项可按“已覆盖（superseded）”处理。

## 仍需推进的核心子任务

RFC #249 在当前仓库内仍主要跟踪这些实现项：

1. **Transport 适配层收敛**
   - 将本地 IPC 路径稳定切换到 `ipckit` 基础设施
   - 保持现有 `TransportAddress` / `IpcListener` / `FramedChannel` API 兼容

2. **ThreadAffinity + HostDispatcher**
   - 引入/完善 `ThreadAffinity` 调度契约
   - 提供 `StandaloneDispatcher` 参考实现
   - 为各 DCC host crate 后续接入保留稳定 trait 边界

3. **主线程 Pump（后续）**
   - 时间片预算 + 协作式让出执行权
   - 避免 DCC UI 线程卡死

## 与子 issue 的映射

- #251: `dcc-mcp-transport` 适配层切片
- #252: `dcc-mcp-process` 线程亲和调度切片
- #253: 主线程 pump（时间片）

## 上游依赖（ipckit）

本 RFC 的完整闭环仍依赖上游能力演进（例如 ThreadAffinity 与 TaskManager 深度集成）。在上游稳定前，仓库内采用分层适配策略，先保证 API 与行为可持续演进。
