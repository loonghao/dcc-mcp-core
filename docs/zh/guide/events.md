# 事件系统

DCC-MCP-Core 提供线程安全的发布/订阅事件系统 `EventBus`，使用 Rust 的 `DashMap` 和 `parking_lot` 实现。

## 创建 EventBus

```python
from dcc_mcp_core import EventBus

bus = EventBus()
```

## 订阅事件

```python
def on_action_done(**kwargs):
    print(f"动作完成: {kwargs}")

# 订阅 — 返回订阅者 ID
sub_id = bus.subscribe("action.completed", on_action_done)
```

## 发布事件

```python
# 发布事件，传递关键字参数
bus.publish("action.completed", action_name="create_sphere", success=True)
```

## 取消订阅

```python
# 使用订阅者 ID 取消订阅
removed = bus.unsubscribe("action.completed", sub_id)  # True 表示已找到并移除
```

## 完整示例

```python
from dcc_mcp_core import EventBus

bus = EventBus()

def on_before_execute(**kwargs):
    print(f"即将执行: {kwargs.get('action_name')}")

def on_after_execute(**kwargs):
    print(f"已执行: {kwargs.get('action_name')}, 成功={kwargs.get('success')}")

bus.subscribe("action.before_execute", on_before_execute)
bus.subscribe("action.after_execute", on_after_execute)

bus.publish("action.before_execute", action_name="create_sphere")
bus.publish("action.after_execute", action_name="create_sphere", success=True)
```

## 建议的事件名称

| 事件 | 说明 |
|------|------|
| `action.before_execute` | 执行动作之前 |
| `action.after_execute` | 执行动作之后 |
| `action.error` | 动作执行出错 |
| `skill.loaded` | 技能包已加载 |
| `skill.scan_complete` | 技能目录扫描完成 |
| `registry.action_registered` | 新动作已注册 |
