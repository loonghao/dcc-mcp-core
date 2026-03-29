---
layout: home

hero:
  name: DCC-MCP-Core
  text: AI ↔ DCC 桥梁
  tagline: DCC 模型上下文协议生态系统的基础库。Rust 驱动核心，零依赖连接 AI 与 Maya、Blender、Houdini 等。
  image:
    src: /logo.svg
    alt: DCC-MCP-Core
  actions:
    - theme: brand
      text: 快速开始
      link: /zh/guide/getting-started
    - theme: alt
      text: GitHub
      link: https://github.com/loonghao/dcc-mcp-core

features:
  - icon: ⚡
    title: Rust 驱动核心
    details: 所有核心逻辑由 Rust 通过 PyO3 实现，零 Python 运行时依赖，极致性能。
  - icon: 🎯
    title: ActionRegistry
    details: 线程安全的动作注册与查询，基于 DashMap 的无锁并发读取。
  - icon: 🔌
    title: Skills 技能包
    details: 将任何脚本（Python、MEL、MaxScript、BAT、Shell）零代码注册为 MCP 工具。
  - icon: 📡
    title: EventBus 事件总线
    details: 线程安全的发布/订阅事件系统，实现组件间解耦通信。
  - icon: 🌐
    title: MCP 协议类型
    details: 完整的 MCP 协议类型定义：Tools、Resources、Prompts。
  - icon: 🔄
    title: 类型包装器
    details: RPyC 兼容的类型包装器，确保远程调用中的类型安全。
---
