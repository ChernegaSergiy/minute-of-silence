# Minute of Silence v0.11.5

This release makes the daily ceremony schedule easier to understand and control, improves tray-based ceremony controls, fixes state updates around nearby remembrance dates and ceremony skipping, and strengthens Linux Flatpak builds.

## New Features

- **Ceremony Schedule Status:** Added a prominent status card to Settings showing the next ceremony state with direct controls for skipping or restoring the next ceremony.

## Improvements & Changes

- **System Tray Ceremony Controls:** Updated the system tray menu with dynamic labels and actions for skipping or restoring the next ceremony.
- **Flatpak Build Reliability:** Flatpak builds now install optional platform-specific npm packages, ensuring Linux native modules such as `lightningcss` are available during the frontend build.
- **Dependency Updates:** Updated Tauri, frontend, Rust, and development dependencies, including Vite, React, Tokio, Rodio, notification tooling, and linting packages.

## Bug Fixes

- **Nearby Date Display State:** Showing a nearby remembrance date during a ceremony no longer marks the Settings form as modified.
- **Ceremony Skip Status:** The frontend now receives a status update event when the next ceremony is skipped or restored, keeping the displayed state synchronized with the backend.

## Installation

Download the updated installers for your platform:

- **Windows:** `.msi` installer, standalone `.exe`, or via the Microsoft Store (MSIX).
- **macOS:** `.dmg` (Intel + Apple Silicon).
- **Linux:** `.deb` package, `AppImage`, Snap Store, or Flatpak.

## License

This project remains licensed under the CSSM Unlimited License v2.0 (CSSM-ULv2).
