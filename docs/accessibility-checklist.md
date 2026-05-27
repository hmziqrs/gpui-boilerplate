# Accessibility Checklist

Use this checklist for every UI change before merge.

## Keyboard And Focus

- Every interactive control is reachable by keyboard only.
- Focus order matches visual reading order.
- Focus is visible on all controls, including icon-only buttons.
- Escape closes transient surfaces (launcher/dialogs) and returns focus predictably.
- Global shortcuts do not block normal text input behaviors.

## Semantics And Labels

- Every icon-only action has a readable text label/tooltip equivalent.
- Form controls have clear labels and helper/error text.
- Button text describes the action, not just "OK" or "Click".
- Notification actions use explicit labels.

## Color And Contrast

- Body text meets WCAG AA contrast against background.
- Muted text remains readable in light and dark themes.
- State is not communicated by color alone; add text/icon cues.

## Motion And Feedback

- Time-based transitions are subtle and do not block interaction.
- Loading/progress states are visible for async work.
- Background task completion/failure is visible in status/notifications.

## Screen Reader / Platform Bridge

- Verify GPUI accessibility bridge behavior on target platforms.
- If metadata gaps are found, track AccessKit integration points.
- Diagnostics includes accessibility capability status.

## Manual QA Passes

- macOS: keyboard-only walkthrough for launcher, sidebar, settings, notifications.
- Windows: keyboard-only walkthrough and menu traversal.
- Linux: keyboard-only walkthrough and menu traversal.

## Done Criteria

- All checklist items pass or have an explicit tracked exception.
- Any exception includes owner, scope, and follow-up milestone.
