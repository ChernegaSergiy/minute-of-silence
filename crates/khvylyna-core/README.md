# khvylyna-core

Shared configuration models, validation, schema migrations, and contracts for **Minute of Silence** ([Хвилина мовчання](https://github.com/ChernegaSergiy/minute-of-silence)).

`khvylyna-core` serves as the portable, single source of truth for ceremony settings, audio block pipelines, voices, and memorial dates. It is designed to run in both desktop applications (Tauri / full OS) and resource-constrained embedded / IoT environments (`no_std` + `alloc`, e.g., ESP32 running Rust).

## Features

- **Embedded & IoT Ready (`no_std`)**: Can be compiled without standard library dependencies (`alloc` only), making it suitable for microcontrollers and standalone hardware devices.
- **Single Source of Truth**: Houses all core types:
  - `Settings`: Ceremony activation, scheduling rules, NTP server, volume, player pause/resume controls.
  - `AudioPreset` & `PresetBlock`: Audio pipelines (announcement, bell, metronome, silence, ending, anthem).
  - `AnnouncementVoice` & `AnthemVoice`: Supported announcer recordings and voice capabilities (`VoiceCapabilities`).
  - `PersonalDate`: Personal memorial dates with date bounds validation.
  - `StatusSnapshot` & `AppInitialState`: Ceremony runtime state representation.
- **Validation & Migration**:
  - Validates audio block sequences, volume boundaries, and scheduling grace windows.
  - Backward-compatible schema migration (`Settings::migrate`) across configuration schema versions.
- **Interoperability**:
  - JSON serialization/deserialization with `serde`.
  - Export and import bundles (`ExportBundle`, `import_bundle_from_json`, `export_bundle_to_json`) omitting device-local fields.
  - TypeScript type definition export via `ts-rs` (`types.gen.ts`).

## Cargo Features

| Feature | Description |
|---|---|
| `std` *(default)* | Enables `std`, `serde_json`, and JSON export/import helpers. |
| `alloc` | Enables `alloc` support without `std` for `no_std` embedded/IoT environments. |
| `ts` | Enables `ts-rs` derives to generate TypeScript interfaces for web frontends. |

## Usage

### As a standard Rust dependency

Add to `Cargo.toml`:

```toml
[dependencies]
khvylyna-core = { path = "path/to/crates/khvylyna-core" }
```

### For embedded / IoT targets (`no_std`)

Disable default features and enable `alloc`:

```toml
[dependencies]
khvylyna-core = { path = "path/to/crates/khvylyna-core", default-features = false, features = ["alloc"] }
```

### Example: Validation & Migrations

```rust
use khvylyna_core::{AudioPreset, PresetBlock, Settings};

let mut settings = Settings::default();
settings.migrate();
assert!(settings.validate().is_ok());

// Inspect blocks for a built-in preset
let blocks = AudioPreset::VoiceMetronome.blocks();
assert_eq!(blocks, &[PresetBlock::Announcement, PresetBlock::Metronome]);
```

## License

Licensed under the same terms as the parent repository (see [LICENSE](../../LICENSE)).
