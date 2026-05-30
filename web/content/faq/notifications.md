---
question: "How do desktop notifications work?"
description: "gpui-starter uses native OS notifications with automatic backend selection and in-app toasts as fallback."
category: "Features"
order: 14
---

Notifications go through a three-tier delivery system. The app picks the best available backend at startup and falls back automatically if something goes wrong.

## Backend selection

On macOS, gpui-starter uses the **user-notify** crate to talk directly to `UNUserNotificationCenter`. This gives you real native banners with interactive actions and reply fields. If the primary backend is unavailable (for example, when running outside a bundled app), it falls back to **notify-rust**, which uses D-Bus/libnotify on Linux and the WinRT API on Windows. When neither native backend can deliver, the app shows an **in-app toast** inside the window instead.

## Notification inbox

Every notification attempt is recorded in a persistent inbox, capped at 200 entries. The inbox tracks the backend used, whether it was delivered natively, and any errors that occurred. You can browse the history, mark items as read, or clear the list from the notifications page. The inbox data survives app restarts.

## Permissions

On macOS the app checks `UNAuthorizationStatus` at startup and can prompt the user with the native permission dialog. If permission is denied, the app gracefully degrades to in-app toasts. The settings page includes a button to open System Settings so users can re-enable notifications. On Linux and Windows the notify-rust backend sends without an explicit permission step.
