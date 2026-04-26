# Vocoflow

Turn your voice into ready-to-paste text in seconds with a fast, privacy-friendly desktop dictation app built in Rust.

> ⚠️ **Hobby project note:** Vocoflow is currently a hobby project. To keep shipping features quickly, official code-signing/notarization is not fully set up yet.

## Quick Start

### Requirements

- A working microphone (uses system default mic).

### Install from GitHub Releases (recommended)

1. Open the GitHub repo’s **Releases** tab.
2. Download the latest build for your OS:
   - **Windows:** `vocoflow-<version>-windows-installer.exe`
   - **macOS:** `vocoflow-<version>-macos.dmg`
   - **Linux:** `vocoflow-<version>-linux.AppImage`
3. Install/run using the platform steps below.

### Upcoming package managers (placeholders)

#### winget (coming soon)

```powershell
winget install vocoflow
```

#### brew (coming soon)

```bash
brew install vocoflow
```

## Security & Release Trust Checks (CI/CD)

- Multi-OS build + test validation (Windows/macOS/Linux).
- Windows installer quiet-mode validation (`/S`) for package manager compatibility.
- Windows Defender malware scan on release installer artifacts.
- SHA256 checksum generation for published artifacts (used by winget/brew style verification).
- Optional VirusTotal upload scan when `VIRUSTOTAL_API_KEY` is configured.
- Trivy supply-chain filesystem scan for high/critical findings.
- WinGet publish automation opens PR on new tag creation for package id `mayankgujrathi.vocoflow`.

### Build from source

```bash
cargo build --release
```

### Run from source

```bash
cargo run --release
```

### Use

1. Press ``Ctrl + ` `` (Windows/Linux) or ``Command + ` `` (macOS) to start recording.
2. Speak normally.
3. Press the same hotkey again to stop.
4. Transcribed text is copied and typed into the active text field.

## Unsigned Install Guidance (Windows/macOS/Linux)

Because this is a hobby project and binaries may be unsigned on some platforms, you may see OS security warnings.

### Windows (Unknown Publisher / SmartScreen)

1. Launch installer.
2. If SmartScreen appears, click **More info**.
3. Click **Run anyway**.

### macOS (Gatekeeper)

1. Try opening the app from Finder.
2. If blocked, right-click app → **Open** → confirm.
3. If still blocked, go to **System Settings → Privacy & Security** and click **Open Anyway** for Vocoflow.

### Linux (AppImage)

1. Make the file executable:
   ```bash
   chmod +x vocoflow-<version>-linux.AppImage
   ```
2. Run it:
   ```bash
   ./vocoflow-<version>-linux.AppImage
   ```

Tip: Always download from the official GitHub Releases page.

## Troubleshooting (Quick)

- **Windows webview issues**: make sure WebView2 runtime is installed.
- **Linux webview issues**: install WebKitGTK stack (`webkit2gtk`, `javascriptcoregtk`, `libsoup3`).
- **macOS WebKit link issues**: build with `RUSTFLAGS="-l framework=WebKit"` when needed.

For full troubleshooting and platform-specific setup details, see the docs below.

## LLM Integration (Optional, OpenAI-compatible)

Vocoflow includes optional LLM-based transcript post-processing for cleaner, context-aware output.

### Why use it (advantages)

- Produces cleaner punctuation/casing automatically.
- Improves readability and polish for notes, docs, chats, and emails.
- Adapts style to the **currently focused app context** (window/app metadata).
- Can move from light cleanup to fully rewritten polished output depending on selected level.

### API compatibility

- Uses an **OpenAI-compatible API** endpoint (`llm_base_url`).
- Works with providers/local gateways that expose OpenAI-style chat completion APIs.

### Default behavior

- By default, LLM reformatting is **not enabled** (`reformatting level = none`).
- In `none` mode, model calls are generally bypassed.

## Settings App Options

Vocoflow settings are grouped into:

### General

- **Start on login**: auto-launch app when you sign in.

### Logging

- **App log max lines**: max retained lines in application log.
- **Trace file limit**: number of trace files kept.
- **Enable debug logs**: turn on verbose debug logging.

### Transcription

- **Model cache TTL (secs)**: cache duration for model metadata.
- **User dictionary**: custom words and vocabulary.
- **Reformatting level**: controls transcript post-processing behavior.
- **LLM base URL**: OpenAI-compatible endpoint.
- **LLM model name**: model identifier to call.
- **LLM API key**: optional provider key (if required).
- **LLM custom prompt**: extra instructions for rewriting behavior.

### Reformatting levels explained

- **none**: keep transcript unchanged; usually skips model calls.
- **minimal**: tiny edits only (punctuation/casing/minor clarity).
- **normal**: improve readability/grammar while preserving intent.
- **freeform**: most polished app-context-adapted output; wording can change more.

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Settings and Logging](docs/SETTINGS_AND_LOGGING.md)
- [Build and Release](docs/BUILD_AND_RELEASE.md)
- [Licensing and Acknowledgments](docs/LICENSES.md)

## License

MIT — see [LICENSE](LICENSE).
