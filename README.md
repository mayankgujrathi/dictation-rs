# dictation-rs: Voice Dictation

Turn your voice into ready-to-paste text in seconds with a fast, privacy-friendly desktop dictation widget built in Rust. `dictation-rs` captures microphone input, runs local model transcription, and sends the result directly to your clipboard and active text field so you can stay in flow while writing.

## Features

- **Minimal UI**: Compact overlay widget with a rounded black background
- **Lightweight**: Minimal footprint and blazing-fast execution.
- **Push-to-Toggle Recording**: Press ``Ctrl + ` `` (Windows/Linux) or ``Command + ` `` (macOS) to start or stop recording.
- **Live Volume Visualization**: Real-time waveform-style level bars while recording.
- **Tray Integration**: System tray icon with an `Exit` action for clean shutdown.

## Requirements

- Rust (latest stable)
- A working microphone (uses system default mic)

## Building

```bash
cargo build --release
```

### Windows icon embedding note

- Windows executable icon embedding is handled via `winres` in `build.rs` using `assets/activity.ico`.
- Local Windows builds should have Visual Studio Build Tools / Windows SDK installed.
- GitHub Actions Windows workflows (`CI/CD` and `Release`) initialize the MSVC environment via `ilammy/msvc-dev-cmd@v1` before build.

The binary will be created at `target/release/dictation.exe`.

## CI/CD packaging outputs

GitHub Actions now builds installer/package-style artifacts for each platform:

- **Windows:** `dictation-<version>-windows-installer.exe` (NSIS installer)
- **Linux:** `dictation-<version>-linux.AppImage`
- **macOS:** `dictation-<version>-macos.dmg` (drag-and-drop `.app` installer style)

Notes:
- `release.yml` publishes these as GitHub Release assets for tags.
- `ci-cd.yml` produces the same artifacts in the workflow artifact `dist/` for branch/PR validation.

## Optional signing/notarization (Phase 2)

The release workflow includes optional, secret-guarded signing steps that activate only when secrets are configured.

### Windows signing secrets

- `WIN_CERT_PFX_BASE64` - Base64-encoded code-signing `.pfx`
- `WIN_CERT_PASSWORD` - Password for the `.pfx`

### macOS signing/notarization secrets

- `MACOS_CERT_P12_BASE64` - Base64-encoded Developer ID Application cert (`.p12`)
- `MACOS_CERT_PASSWORD` - Password for cert import
- `KEYCHAIN_PASSWORD` - Temporary CI keychain password
- `APPLE_ID` - Apple ID used for notarization
- `APPLE_APP_SPECIFIC_PASSWORD` - App-specific password for Apple ID
- `APPLE_TEAM_ID` - Apple Developer Team ID

### Linux authenticity (optional) secrets

- `LINUX_GPG_PRIVATE_KEY` - Base64-encoded GPG private key
- `LINUX_GPG_PASSPHRASE` - GPG key passphrase

When Linux GPG secrets are present, CI also publishes:

- `dictation-<version>-linux.AppImage.sha256.asc`

## Usage

1. Start the application:
   ```bash
   cargo run --release
   ```
   or run the built binary directly:
   ```bash
   target/release/dictation.exe
   ```

2. Use ``Ctrl + ` `` (Windows/Linux) or ``Command + ` `` (macOS) to toggle recording on.

3. While recording, the overlay appears near the bottom-center of the primary monitor and shows live input level bars.

4. Press the same key again to stop recording. The app finalizes the captured audio and runs transcription.

5. The transcribed text is copied to your clipboard and typed into the currently focused text field.

## Logging & Tracing

- Base path (per OS):
  - **Windows:** `C:\Users\<username>\AppData\Roaming\dictation\dictation\`
  - **macOS:** `/Users/<username>/Library/Application Support/com.dictation.dictation/`
  - **Linux:** `/home/<username>/.local/share/dictation/dictation/`
- Application logs are written to:
  - `<base_path>/logs/application.log`
- Trace files are written to:
  - `<base_path>/logs/traces/`
  - File format: `trace-<timestamp>.json` (Chrome Trace Event format)

### Viewing traces graphically

- Open trace files in **Perfetto UI** (recommended): https://ui.perfetto.dev
- Or open with Chromium tracing at `chrome://tracing`
- Load a file from `<base_path>/logs/traces/trace-<timestamp>.json`

### Retention / rotation

- `application.log` keeps only the last **1000 lines** by default.
- Trace retention keeps only the latest **100 trace files** by default.

Both can be configured via `<base_path>/settings.json` (sibling of `logs/`).

### Log levels

- Default logging level is `info` (includes `info`, `warn`, `error`).
- Debug logging can be enabled via `settings.json`.

### settings.json (logging)

- Path: `<base_path>/settings.json`
- Location relative to logs:
  - `settings.json` lives next to `logs/`
  - e.g. `<base_path>/settings.json` and `<base_path>/logs/`

Example:

```json
{
  "logging": {
    "app_log_max_lines": 1000,
    "trace_file_limit": 100,
    "enable_debug_logs": false
  },
  "transcription": {
    "built_in_dictionary": [],
    "user_dictionary": [],
    "model_cache_ttl_secs": 600,
    "transcript_reformatting_level": "none",
    "llm_api_key": null,
    "llm_base_url": "http://localhost:11434/v1",
    "llm_model_name": "",
    "llm_custom_prompt": "Rewrite the transcript according to the requested reformatting level and active application context while preserving user intent."
  }
}
```

`transcript_reformatting_level` controls post-processing behavior:
- `none` (default): skip model call, return normalized transcript as-is
- `minimal`: small readability fixes
- `normal`: context-aware rewrite while preserving intent
- `freeform`: advanced context-targeted output

Notes:
- The app creates this file with defaults if it is missing.
- Logging settings are refreshed at runtime (approximately every second).


## Architecture

```
src/
├── main.rs          - Application entry point, runtime/bootstrap, hotkey listener, and app wiring
├── lib.rs           - Library exports for core modules
├── audio.rs         - Audio capture (cpal), buffering, and WAV writing
├── logging.rs       - File logging/tracing initialization, retention, and runtime log-level refresh
├── settings.rs      - Persistent settings loading/defaults and runtime access
├── tray.rs          - Tray icon/menu integration and graceful shutdown signaling
└── app/
    ├── mod.rs       - VoiceApp state and high-level app module composition
    ├── constants.rs - Shared UI constants (window size/history limits)
    ├── positioning.rs - Overlay window placement logic
    ├── render.rs    - egui rendering and UI state transitions
    ├── state.rs     - UI state definitions for download/recording/transcription phases
    └── workers.rs   - Background workers (model download/readiness and transcription workflow)
```

### Key Components

- **cpal**: Cross-platform microphone capture/input stream handling
- **eframe/egui**: Always-on-top transparent overlay UI and visualization rendering
- **winit**: Window/monitor integration used for positioning behavior
- **hound**: WAV encoding for captured audio segments
- **rdev**: Global hotkey listener (``Ctrl + ` `` on Windows/Linux, ``Command + ` `` on macOS)
- **tokio**: Async runtime for background workers and periodic tasks
- **tray-icon**: Native system tray menu and exit action
- **single-instance**: Prevents running duplicate app instances
- **transcribe-rs**: Local speech-to-text inference/model orchestration
- **reqwest**: Model/network download support used by transcription pipeline
- **arboard**: Clipboard write integration for transcribed output
- **enigo**: Simulated keystroke typing into the active text field
- **active-win-pos-rs**: Active window metadata used for typing/placement context
- **tracing + tracing-subscriber + tracing-appender + tracing-chrome**: Structured app logging and Chrome Trace generation
- **serde/serde_json**: `settings.json` serialization/deserialization

## License

MIT
