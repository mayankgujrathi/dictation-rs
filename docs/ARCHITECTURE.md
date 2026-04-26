# Architecture

This page contains the technical architecture details moved from the main README.

## Source Layout

```text
src/
├── main.rs / lib.rs      # app entry and module exports
├── audio.rs              # microphone capture and WAV writing
├── settings.rs           # persisted runtime settings
├── logging.rs            # logs + trace setup/retention
├── llm.rs                # optional LLM post-processing
├── tray.rs / autostart.rs# tray lifecycle + startup integration
├── app/                  # overlay app state + workers
└── settings_window/      # settings webview bridge/runtime
```

## Runtime Flow

1. User toggles recording hotkey.
2. Audio is captured and transcribed locally.
3. Optional post-processing runs based on reformatting level.
4. If enabled, LLM receives transcript + focused-app context (window/app metadata).
5. Final text is copied and typed into the active field.

## Key Components

- **cpal**: Cross-platform microphone capture/input streams
- **eframe/egui**: Overlay UI rendering and visualizations
- **winit + wry**: Native windowing and settings webview
- **hound**: WAV encoding for captured audio
- **rdev**: Global hotkey listener
- **tokio**: Async runtime for background workers
- **tray-icon**: Native system tray integration
- **single-instance**: Prevents duplicate app instances
- **transcribe-rs**: Local speech-to-text inference/model orchestration
- **reqwest**: Model/network download support
- **arboard**: Clipboard integration
- **enigo**: Simulated typing into active text field
- **active-win-pos-rs**: Active window metadata integration
- **tracing + tracing-subscriber + tracing-appender + tracing-chrome**: Structured logs and trace generation
- **serde/serde_json**: `settings.json` serialization/deserialization

## Related Docs

- [Settings and Logging](SETTINGS_AND_LOGGING.md)
- [Build and Release](BUILD_AND_RELEASE.md)
- [Licensing and Acknowledgments](LICENSES.md)
