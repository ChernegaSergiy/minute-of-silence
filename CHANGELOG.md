# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

## [0.2.0] - 2026-03-29

### Added
- Dedicated "About" tab in the user interface with project information.
- Manual NTP synchronization button for immediate time correction.
- Visual indicator for unsaved settings (color change and asterisk).
- Official application logo in the "About" section and updated system icons.
- Clickable repository link using `tauri-plugin-shell`.
- Integrated audio playback engine using `rodio` (backend-driven).

### Changed
- Switched to a high-quality monospace font stack (Ubuntu Mono, JetBrains Mono).
- Expanded tab buttons to full width for a more balanced layout.
- Reduced default late-start grace window from 5 minutes to 1 minute.
- Optimized internal scheduler loop frequency to 1 second.

### Fixed
- Disabled text selection and browser context menu for a native desktop feel.
- Improved UI status synchronization after saving settings.
- Resolved numerous Rust clippy warnings and formatting issues.

## [0.1.0] - 2026-03-29

### Added
- Initial project scaffold (Tauri 2 + Rust + TypeScript/Vite).
- Daily scheduler with NTP time correction and configurable late-start grace window.
- Five audio presets: Voice+Silence+Bell, Voice+Anthem, Voice+Metronome, Voice+Metronome+Anthem, Metronome-only.
- System Tray icon with context menu (Open, Skip Tomorrow, Quit).
- Persistent JSON settings stored in the platform config directory.
- Windows media pause/resume via `SendInput(VK_MEDIA_PLAY_PAUSE)`.
- Linux media pause/resume via `xdotool` fallback (MPRIS D-Bus planned).
- Visual ceremony overlay (brutalist full-screen indicator).
- `WM_POWERBROADCAST` handling for post-sleep scheduler correction (Windows).
- Autostart on system login via `tauri-plugin-autostart`.
- Structured logging with log rotation via `tauri-plugin-log`.
- CI/CD pipeline on GitHub Actions (lint, test, build for Windows + Linux).
- Conventional Commits enforcement documented in CONTRIBUTING.md.

### Notes
- Audio playback engine is **not yet implemented**; the scheduler fires events
  and the overlay appears, but no sound is produced in this release.
