---
title: "Testing"
description: "Testing strategies for GPUI apps including unit tests, entity tests, and integration harnesses"
---

## Overview

gpui-starter uses two levels of testing: plain `#[test]` for logic-only code and `#[gpui::test]` for tests that need GPUI's app context, async executor, or window support. The `src/testing.rs` module provides fake implementations of external services so tests run without network, filesystem, or OS keyring dependencies.

## Test attribute reference

| Attribute | Use case | Context parameter |
|-----------|----------|-------------------|
| `#[test]` | Pure logic, parsing, validation | None |
| `#[gpui::test]` | Entity operations, globals, async | `&mut TestAppContext` |
| `#[gpui::test]` async | Async tasks, timers, channels | `&mut TestAppContext` |
| `#[gpui::test(iterations = 10)]` | Property testing with random data | `&mut TestAppContext`, `mut StdRng` |

If a test does not need windows or rendering, plain `#[test]` is sufficient. Reserve `#[gpui::test]` for code that calls `cx.new()`, `cx.spawn()`, or reads globals.

## Unit testing entity logic

Test data models, validation, and state transitions without GPUI context. These are standard Rust tests that run fast and need no setup.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_clears_redo_history() {
        let mut model = UndoModel {
            future: vec![sample_entry()],
            ..UndoModel::default()
        };
        model.record(sample_entry());
        assert_eq!(model.past.len(), 1);
        assert!(model.future.is_empty());
    }

    #[test]
    fn rejects_unknown_deep_links() {
        assert!(AppRoute::parse_deep_link("https://example.com").is_err());
        assert!(AppRoute::parse_deep_link("gpui-starter://missing").is_err());
    }
}
```

Tests that touch the filesystem should use `tempfile::tempdir()` for isolated directories:

```rust
#[test]
fn corrupt_config_is_quarantined() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    std::fs::write(&state_file, "{not-json").unwrap();

    let (loaded, err) = load_config(&state_file);

    assert_eq!(loaded, AppConfig::default());
    assert!(err.is_some());
    assert!(!state_file.exists());
    assert!(state_file.with_extension("json.bad").exists());
}
```

## Testing with GPUI's test context

`TestAppContext` provides a single-threaded deterministic executor. Use it for entity creation, updates, subscriptions, and async tasks.

```rust
#[gpui::test]
fn test_entity_state(cx: &mut TestAppContext) {
    let entity = cx.new(|cx| Counter::new(cx));

    let initial = entity.read_with(cx, |counter, _| counter.count);
    assert_eq!(initial, 0);

    entity.update(cx, |counter, cx| {
        counter.count = 42;
        cx.notify();
    });

    let updated = entity.read_with(cx, |counter, _| counter.count);
    assert_eq!(updated, 42);
}
```

For window-dependent tests, open a window and convert to `VisualTestContext`:

```rust
#[gpui::test]
fn test_with_window(cx: &mut TestAppContext) {
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|cx| MyView::new(cx))
        }).unwrap()
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let root = window.root(&mut cx).unwrap();
    // interact with root entity...
}
```

## Async testing

Async tests use `#[gpui::test]` on an `async fn`. Call `cx.run_until_parked()` to flush pending tasks and timers.

```rust
#[gpui::test]
async fn test_async_task(cx: &mut TestAppContext) {
    let entity = cx.new(|cx| MyComponent::new(cx));

    entity.update(cx, |comp, cx| comp.start_background_update(cx));

    // detached tasks don't run until you yield
    let before = entity.read_with(cx, |comp, _| comp.value);
    assert_eq!(before, 0);

    cx.run_until_parked();

    let after = entity.read_with(cx, |comp, _| comp.value);
    assert_eq!(after, 10);
}
```

For tests involving external I/O (real threads, OS sockets), call `cx.executor().allow_parking()` so the executor can block on external events.

## Mocking globals and external services

The `src/testing.rs` module provides fakes for every external dependency. Each fake is a plain struct with no GPUI dependency, making it usable in both `#[test]` and `#[gpui::test]` contexts.

| Fake | Methods | Purpose |
|------|---------|---------|
| `FakeTelemetrySink` | `record_event`, `record_error`, `flush` | Tracks telemetry calls in memory |
| `FakeConnectivityProbe` | `probe()` | Returns `Ok` or `Err` based on `next_ok` field |
| `FakeNotificationBackend` | `send` | Records sent notifications; `fail_send` toggles errors |
| `FakeSecureStorage` | `set`, `get`, `delete` | In-memory secret storage |

Construct a fake, configure it for the test scenario, and pass it to the code under test:

```rust
#[test]
fn fake_notification_backend_success_and_failure() {
    let mut backend = FakeNotificationBackend::default();
    backend.send("hello").expect("send");
    assert_eq!(backend.sent.len(), 1);

    backend.fail_send = true;
    assert!(backend.send("world").is_err());
    assert_eq!(backend.sent.len(), 1); // no new entry on failure
}
```

For GPUI globals, tests can call `set_global` to install a fake and `remove_global` to clean up between test cases. Globals are not permanent for the app lifetime and can be replaced freely in test setups.

## Testing form validation

Form structs derive `Koruma` which provides a `validate()` method. Test validation rules on plain struct instances without GPUI context:

```rust
#[test]
fn empty_name_fails_validation() {
    let mut form = RegistrationForm::default();
    form.name = String::new();
    let result = form.validate();
    assert!(result.is_err());
}

#[test]
fn valid_form_passes() {
    let mut form = RegistrationForm::default();
    form.name = "Ada Lovelace".to_string();
    form.email = "ada@example.com".to_string();
    form.password = "secret".to_string();
    form.phone = "(555) 123-4567".to_string();
    form.website = "https://example.com".to_string();
    assert!(form.validate().is_ok());
}
```

## Testing state transitions and commands

Test action dispatching through a window's focus handle:

```rust
actions!(my_app, [Increment]);

#[gpui::test]
fn test_action_dispatch(cx: &mut TestAppContext) {
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|cx| Counter::new(cx))
        }).unwrap()
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let counter = window.root(&mut cx).unwrap();

    let handle = counter.read_with(&cx, |c, _| c.focus_handle.clone());
    cx.update(|window, cx| {
        handle.dispatch_action(&Increment, window, cx);
    });

    let count = counter.read_with(&cx, |c, _| c.count);
    assert_eq!(count, 1);
}
```

For state machines like `UndoModel`, test each transition in isolation:

```rust
#[test]
fn pop_undo_sets_rejected_reason_when_empty() {
    let mut model = UndoModel::default();
    assert!(model.pop_undo().is_none());
    assert_eq!(model.last_rejected.as_deref(), Some("nothing to undo"));
}
```

## Testing event subscriptions

Subscribe to entity events and verify the handler receives them:

```rust
#[derive(Clone)]
struct ValueChanged { new_value: i32 }

impl EventEmitter<ValueChanged> for MyComponent {}

#[gpui::test]
fn test_event_emission(cx: &mut TestAppContext) {
    let component = cx.new(|cx| {
        cx.subscribe_self(|this, event: &ValueChanged, cx| {
            this.received_value = event.new_value;
            cx.notify();
        });
        MyComponent::default()
    });

    component.update(cx, |_, cx| {
        cx.emit(ValueChanged { new_value: 123 });
    });

    let received = component.read_with(cx, |comp, _| comp.received_value);
    assert_eq!(received, 123);
}
```

## Integration test patterns

Integration tests live in the `tests/` directory. They exercise cross-module behavior such as config persistence, deep link routing, or IPC forwarding:

```rust
// tests/qa_docs.rs
#[test]
fn qa_matrix_contains_core_cases() {
    let content = std::fs::read_to_string("docs/qa-matrix.md").expect("read qa matrix");
    let normalized = content.to_lowercase();
    assert!(normalized.contains("second-instance forwarding"));
    assert!(normalized.contains("open logs folder"));
}
```

For tests that cross process boundaries (single-instance IPC), use `tempdir()` for the queue file and a real local socket with a unique name:

```rust
#[test]
fn forwarded_links_roundtrip_in_order() {
    let dir = tempdir().expect("tempdir");
    let queue = dir.path().join("forward.queue");

    append_forwarded_link(&queue, "gpui-starter://settings");
    append_forwarded_link(&queue, "gpui-starter://notifications");

    let links = drain_forwarded_links(&queue);
    assert_eq!(links, vec![
        "gpui-starter://settings".to_string(),
        "gpui-starter://notifications".to_string()
    ]);
    assert!(drain_forwarded_links(&queue).is_empty());
}
```

## Property testing

Use `#[gpui::test(iterations = N)]` with a `mut rng: StdRng` parameter to run randomized tests:

```rust
#[gpui::test(iterations = 10)]
fn test_counter_random_operations(cx: &mut TestAppContext, mut rng: StdRng) {
    let counter = cx.new(|cx| Counter::new(cx));
    let mut expected = 0i32;

    for _ in 0..100 {
        let delta = rng.random_range(-10..=10);
        expected += delta;
        counter.update(cx, |c, cx| { c.count += delta; cx.notify(); });
    }

    let actual = counter.read_with(cx, |c, _| c.count);
    assert_eq!(actual, expected);
}
```

## Running tests

```bash
# Run all tests
cargo test

# Run tests in a specific module
cargo test routes::tests

# Run a single test by name
cargo test corrupt_config_is_quarantined

# Show println output
cargo test -- --nocapture

# Run with backtrace on failure
RUST_BACKTRACE=1 cargo test
```

## Test organization

Group related tests into submodules within each source file. Use helper functions for common setup:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> UndoEntry {
        UndoEntry {
            kind: UndoKind::ThemeChanged {
                before: ThemeMode::Light,
                after: ThemeMode::Dark,
            },
        }
    }

    #[test]
    fn record_clears_redo_history() {
        let mut model = UndoModel {
            future: vec![sample_entry()],
            ..UndoModel::default()
        };
        model.record(sample_entry());
        assert!(model.future.is_empty());
    }
}
```
