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
- A working microphone

## Building

```bash
cargo build --release
```

The binary will be created at `target/release/dictation.exe`.

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

### Retention / rotation

- `application.log` keeps only the last **1000 lines**.
- Trace retention keeps only the latest **100 trace files**.

### Log levels

- Default logging level is `info` (includes `info`, `warn`, `error`).
- To enable debug logs, set:

```bash
DICTATION_ENABLE_DEBUG_LOGS=true
```

Accepted truthy values: `1`, `true`, `yes`, `on`.


## Architecture

```
src/
├── main.rs          - Application entry point and lifecycle wiring
├── lib.rs           - Shared library exports for app modules
├── audio.rs         - Audio capture pipeline (cpal), processing, and WAV handling
├── tray.rs          - System tray integration and shutdown actions
└── app/
    ├── mod.rs       - eframe/egui overlay state, rendering, and UI events
    └── workers.rs   - Background workers for recording/transcription orchestration
```

### Key Components

- **cpal**: Cross-platform microphone capture
- **eframe/egui**: Always-on-top overlay widget and visualization UI
- **hound**: WAV encoding for captured audio segments
- **rdev**: Global key listener for push-to-toggle recording (`` Ctrl + ` `` on Windows/Linux, ``Command + ` `` on macOS)
- **tray-icon**: Native tray icon/menu integration
- **tokio**: Async runtime for background coordination and non-blocking tasks

## License

MIT
