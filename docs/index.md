---
layout: home

hero:
  name: DCC-MCP-Core
  text: AI ↔ DCC Bridge
  tagline: Foundational library for the DCC Model Context Protocol ecosystem. Rust-powered core with zero dependencies connecting AI with Maya, Blender, Houdini, and more.
  image:
    src: /logo.svg
    alt: DCC-MCP-Core
  actions:
    - theme: brand
      text: Get Started
      link: /guide/getting-started
    - theme: alt
      text: View on GitHub
      link: https://github.com/loonghao/dcc-mcp-core

features:
  - icon: ⚡
    title: Rust-Powered Core
    details: All core logic implemented in Rust via PyO3. Zero Python runtime dependencies, maximum performance.
  - icon: 🎯
    title: ActionRegistry
    details: Thread-safe action registration and lookup using DashMap for lock-free concurrent reads.
  - icon: 🔌
    title: Skills System
    details: Register any script (Python, MEL, MaxScript, BAT, Shell) as MCP tools with zero code via SKILL.md.
  - icon: 📡
    title: EventBus
    details: Thread-safe publish/subscribe event system for decoupled component communication.
  - icon: 🌐
    title: MCP Protocol Types
    details: Full MCP protocol type definitions for Tools, Resources, and Prompts abstractions.
  - icon: 🔄
    title: Type Wrappers
    details: RPyC-compatible type wrappers ensuring type safety in remote procedure calls.
---
