# Event System

DCC-MCP-Core provides a thread-safe publish/subscribe event system via the `EventBus`, implemented in Rust using `DashMap` and `parking_lot`.

## Creating an EventBus

```python
from dcc_mcp_core import EventBus

bus = EventBus()
```

## Subscribing to Events

```python
def on_action_done(**kwargs):
    print(f"Action completed: {kwargs}")

# Subscribe — returns a subscriber ID
sub_id = bus.subscribe("action.completed", on_action_done)
```

## Publishing Events

```python
# Publish an event with keyword arguments
bus.publish("action.completed", action_name="create_sphere", success=True)
```

All registered callbacks for the event name are called with the provided keyword arguments.

## Unsubscribing

```python
# Unsubscribe using the subscriber ID
removed = bus.unsubscribe("action.completed", sub_id)  # True if found
```

## Example: Action Lifecycle Events

```python
from dcc_mcp_core import EventBus

bus = EventBus()

def on_before_execute(**kwargs):
    print(f"About to execute: {kwargs.get('action_name')}")

def on_after_execute(**kwargs):
    print(f"Executed: {kwargs.get('action_name')}, success={kwargs.get('success')}")

# Subscribe
bus.subscribe("action.before_execute", on_before_execute)
bus.subscribe("action.after_execute", on_after_execute)

# Publish (typically done by the action manager layer)
bus.publish("action.before_execute", action_name="create_sphere")
# ... execute action ...
bus.publish("action.after_execute", action_name="create_sphere", success=True)
```

## Suggested Event Names

When building on top of DCC-MCP-Core, consider using these event name conventions:

| Event | Description |
|-------|-------------|
| `action.before_execute` | Before executing an action |
| `action.after_execute` | After executing an action |
| `action.error` | When an action raises an error |
| `skill.loaded` | When a skill package is loaded |
| `skill.scan_complete` | After scanning skill directories |
| `registry.action_registered` | When a new action is registered |
