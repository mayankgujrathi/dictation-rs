# dictation-rs: Voice Dictation

A high-performance Rust-based voice widget that captures microphone input, transcribes audio locally using Whisper, and automatically pipes the text to your clipboard and active text field.

## Features

- **Minimal UI**: Compact overlay widget with a rounded black background
- **Lightweight**: Minimal footprint and blazing-fast execution.
- **Whisper Integration**: High-accuracy speech-to-text.
- **Clean Exit**: Press Escape to stop recording and close the widget
- **Seamless Workflow**: Direct-to-clipboard and active input injection.

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

2. The widget will appear at the top-center of your primary monitor.

3. **Record**: Just start talking - recording begins automatically when the widget launches.

4. **Stop & Save**: Press `Escape` to stop recording and save the audio.

## Architecture

```
src/
├── main.rs    - Application entry point, initializes audio and UI
├── app.rs     - eframe/egui UI, renders volume visualization
└── audio.rs   - Audio capture via cpal, WAV encoding via hound
```

### Key Components

- **cpal**: Cross-platform audio input
- **eframe/egui**: Lightweight immediate-mode GUI
- **hound**: WAV file encoding

## License

MIT
