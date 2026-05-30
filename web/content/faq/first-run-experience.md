---
question: "Is there a first-run or onboarding experience?"
description: "gpui-starter detects first launches and can show a welcome screen or setup wizard."
category: "Features"
order: 15
---

The `first_run` module provides a simple onboarding flow that runs once after a fresh install or when the user resets app state.

## How detection works

On startup, the app loads its persisted config from a JSON state file. The `first_run_completed` field in `AppConfig` defaults to `false`. The `first_run::is_pending` function checks this flag. If no config file exists yet (brand new install), defaults are used, so the first-run flow activates automatically.

## What happens on first run

When `is_pending` returns true, the home page renders an inline setup panel above the normal content. This panel lets users pick a locale, toggle native notifications, and press "Finish setup" to dismiss it. Calling `first_run::complete` sets the flag to true and persists the config immediately.

## Customizing the welcome experience

You can modify the setup panel in `views/home.rs` to add your own steps, such as theme selection, data import, or a feature walkthrough. The `first_run` module exposes three functions:

- `is_pending(cx)` checks whether the onboarding should show
- `complete(cx)` marks the flow as done and saves the config
- `reset(cx)` sets the flag back to false for testing

To retrigger the flow during development, call `first_run::reset` from a debug action or the diagnostics page.
