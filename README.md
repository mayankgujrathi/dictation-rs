# dictation-rs: Voice Dictation

A high-performance Rust-based voice widget that captures microphone input, transcribes audio locally using Whisper, and automatically pipes the text to your clipboard and active text field.

## Features

- **Minimal UI**: Compact overlay widget with a rounded black background
- **Lightweight**: Minimal footprint and blazing-fast execution.
- **Push-to-Toggle Recording**: Press `Right Alt (AltGr)` to start or stop recording.
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

1. Run the application:
   ```bash
   cargo run --release
   ```
   Or:
   ```bash
   ./target/release/dictation.exe
   ```

2. The widget will appear near the bottom-center of your primary monitor while recording.

3. Press `Right Alt (AltGr)` to start recording.

4. Press `Right Alt (AltGr)` again to stop and save the audio.


## Architecture

```
src/
├── main.rs    - Application entry point, runtime setup, global hotkey listener
├── app.rs     - eframe/egui overlay UI and live volume rendering
├── audio.rs   - Audio capture via cpal, resampling, WAV encoding via hound
└── tray.rs    - System tray icon/menu and exit event handling
```

### Key Components

- **cpal**: Cross-platform audio input
- **eframe/egui**: Lightweight immediate-mode GUI
- **hound**: WAV file encoding
- **rdev**: Global keyboard listener for `AltGr` toggle
- **tray-icon**: Native system tray icon and menu support
- **tokio**: Runtime for background task orchestration

## License

MIT
