# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-05-15

### Added

- **Multi-page architecture** with sidebar navigation and page routing via GPUI
- **21 built-in themes** with live hot-reloading and custom theme support
- **Internationalization (i18n)** supporting English and Chinese (zh-CN) via `es-fluent`
- **Form validation** with `gpui-form` derive macros and `koruma` validation rules
- **Command launcher** (Cmd+K) with fuzzy search across all app actions
- **macOS system tray** integration with app icon and quick-access menu
- **Secure credential storage** via OS keychain (`keyring` crate)
- **SQLite database** integration with `rusqlite` for local data persistence
- **Animated app preview** component with Three.js wireframe scenes
- **Documentation site** built with Astro Starlight
- **Custom marketing landing page** with 3D animations and glassmorphism design
- **Privacy policy** and **terms of use** pages

### Changed

- Moved web sources from `src/web` to `web` directory for cleaner project structure
- Enabled `apple-native` feature for keyring on macOS
- Switched to blocking `reqwest` for connectivity checks

### Fixed

- Animation now pauses on hover for better UX
- Added logging to secure storage operations for debugging
