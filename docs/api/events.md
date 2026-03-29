# Events API

## EventBus

`dcc_mcp_core.EventBus` — Thread-safe publish/subscribe event system, implemented in Rust using DashMap and parking_lot.

```python
from dcc_mcp_core import EventBus
```

### Constructor

```python
bus = EventBus()
```

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `subscribe(event_name, callback)` | `int` | Subscribe to event, returns subscriber ID |
| `unsubscribe(event_name, subscriber_id)` | `bool` | Unsubscribe by ID, returns True if found |
| `publish(event_name, **kwargs)` | `None` | Publish event, calling all subscribers |

### Usage

```python
from dcc_mcp_core import EventBus

bus = EventBus()

def on_action_done(**kwargs):
    print(f"Action: {kwargs.get('action_name')}, success: {kwargs.get('success')}")

# Subscribe
sub_id = bus.subscribe("action.completed", on_action_done)

# Publish
bus.publish("action.completed", action_name="create_sphere", success=True)

# Unsubscribe
bus.unsubscribe("action.completed", sub_id)
```

### Error Handling

If a subscriber callback raises an exception, it is logged via `tracing::error` but does not stop other subscribers from being called.
