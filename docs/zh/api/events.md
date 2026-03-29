# 事件 API

## EventBus

线程安全的发布/订阅事件系统，由 Rust 实现。

```python
from dcc_mcp_core import EventBus

bus = EventBus()
```

### 方法

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `subscribe(event_name, callback)` | `int` | 订阅事件，返回订阅者 ID |
| `unsubscribe(event_name, subscriber_id)` | `bool` | 取消订阅 |
| `publish(event_name, **kwargs)` | `None` | 发布事件 |

### 使用示例

```python
def handler(**kwargs):
    print(f"事件数据: {kwargs}")

sub_id = bus.subscribe("action.completed", handler)
bus.publish("action.completed", action_name="create_sphere", success=True)
bus.unsubscribe("action.completed", sub_id)
```
