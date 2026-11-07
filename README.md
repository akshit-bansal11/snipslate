# Snipslate

Snip any part of your screen and **translate it** with your own AI key, or **copy its text**. Like Snipping Tool, but for text.

Snipslate is a native tray app for **Windows, macOS and Linux**, written in Rust with egui.

- **Copy text** (`Ctrl+Alt+C`): drag a box and the text in it is copied to the clipboard. Uses the operating system's text recognition (Windows and macOS built-in, Tesseract on Linux), so it works offline and costs nothing.
- **Translate** (`Ctrl+Alt+T`): drag a box and the snip goes to the AI provider you chose. The translation appears in a small popup next to the snip with a **Copy** button.

Default hotkeys: `Ctrl+Alt+T` / `Ctrl+Alt+C` on Windows and macOS; `Ctrl+Alt+Shift+T` / `Ctrl+Alt+Shift+C` on Linux. You can change them in Settings.

Snipslate runs in the system tray. Right-click the tray icon for the menu; double-click it to translate.

## Install

Download the file for your system from [Releases](../../releases) and check it against `SHA256SUMS.txt` from the same release.

| System | File | How to run |
| --- | --- | --- |
| Windows 10 (1903) or later, 64-bit | `Snipslate-<version>-windows-x64.exe` | Run it. No installer. |
| macOS (Intel and Apple silicon) | `Snipslate-<version>-macos-universal.zip` | Unzip, move `Snipslate.app` to Applications, then run it. See below. |
| Linux, 64-bit | `Snipslate-<version>-linux-x64.tar.gz` | Extract, run `snipslate`. See below. |

Windows SmartScreen may warn the first time, because the exe isn't code-signed. Check the hash:

```powershell
(Get-FileHash .\Snipslate-<version>-windows-x64.exe -Algorithm SHA256).Hash
```

To start it with Windows, put a shortcut to the exe in `shell:startup`.

### macOS

The app is ad-hoc signed only, not notarised. Remove the quarantine flag before the first run:

```sh
xattr -dr com.apple.quarantine Snipslate.app
```

On the first snip, macOS asks for **Screen Recording** permission for Snipslate. Without it, snips capture only the wallpaper, and Snipslate cannot detect that. In the hotkeys, `Win` means `Cmd`.

### Linux

The tarball contains one binary and `LICENSE`. Runtime libraries it needs: `libgtk-3-0`, `libayatana-appindicator3-1`, `libxdo3`, `libdbus-1-3`, and `tesseract-ocr` (English). Package names are for Debian and Ubuntu and are not checked on other distributions.

- API keys need a Secret Service provider, such as gnome-keyring.
- On Wayland, screen capture goes through `xdg-desktop-portal`, and the desktop may ask for permission.
- GNOME needs the AppIndicator extension to show the tray icon.
- **Wayland has no global hotkeys.** Bind `snipslate --translate` and `snipslate --copy` in your desktop's keyboard shortcut settings. These flags work on every OS.
- OCR for Copy text and Text-only mode is **English only**.

## Set up a provider

Open **Settings** from the tray menu, pick a provider, paste an API key and press **Test**.

| Provider | Base URL | Default model |
| --- | --- | --- |
| Anthropic | `https://api.anthropic.com/v1` | `claude-opus-5-5` |
| OpenAI | `https://api.openai.com/v1` | `gpt-5-mini` |
| OpenRouter | `https://openrouter.ai/api/v1` | `openai/gpt-5-mini` |
| Custom | any OpenAI-compatible server (Ollama, LM Studio, vLLM…) | none; enter yours |

The model field is free text, so you can use any model your account offers. The image is sent to the model, so pick one that can read images. For a text-only model, tick **Send text only**: Snipslate reads the text offline first and sends just the text.

A custom base URL must use `https://`. Plain `http://` is allowed only for a server on this machine (`localhost`), so a key never crosses a network unencrypted. A key is optional for a local server.

**Keys saved by the old Windows-only version (.NET) are not read by this version. Re-enter them in Settings.**

## Privacy

- **Copy text never leaves your computer.**
- **Translate** sends the snipped image (or its text, with *Send text only*) to the provider you configured, and nowhere else. Nothing is sent until you finish a snip.
- API keys are stored in the system keystore: **Windows Credential Manager**, the **macOS Keychain**, or the **Linux Secret Service**. Look for `Snipslate` entries. They are not stored in a file.
- Settings live in a `settings.json` file and contain no keys. Its location:
  - Windows: `%APPDATA%\Snipslate\settings.json`
  - macOS: `~/Library/Application Support/Snipslate/settings.json`
  - Linux: `$XDG_CONFIG_HOME/snipslate/settings.json`, or `~/.config/snipslate/settings.json`
- No telemetry, no analytics, no update checks.

## Netflix and other protected video

Protected video (Netflix, Disney+, Prime Video and other DRM players) always captures as a black box. That's the copy protection working as designed, and every screenshot tool sees the same thing. Snipslate doesn't try to get around it. It notices a black snip and tells you instead of sending a wasted, paid request. For video, use the player's own subtitle languages.

## Known limitation: complex scripts in the popup

The popup doesn't shape text. Arabic, Urdu, Hindi, Bengali and Thai translations show **unshaped** (letters not joined, marks misplaced). The **Copy** button still copies the correct text, so use it for those languages.

## Develop

Needs Rust **1.89** or later (`rust-version` in `Cargo.toml`), with `rustfmt` and `clippy`.

| Path | What |
| --- | --- |
| `src/main.rs` | Argument dispatch: tray, `--translate`, `--copy`, `--settings`, `--message` |
| `src/tray.rs` | The tray process: tray icon, menu, global hotkeys. It starts snip, settings and message windows as child processes. |
| `src/ui/` | The snip overlay and result window, settings and message windows (egui) |
| `src/ocr/` | Text recognition per OS: `win.rs`, `macos.rs`, `linux.rs` |
| `src/translator.rs`, `src/settings.rs`, `src/hotkey.rs`, `src/imaging.rs`, `src/clip.rs` | Provider calls, settings and keystore, hotkey parsing, image checks, clipboard |
| `scripts/check.ps1` | The quality gate: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`. `-Ci` runs `fmt --check` instead of formatting |
| `.github/workflows/` | `ci.yml` runs the gate on Windows, macOS and Linux; `release.yml` builds and publishes a release for a `v*` tag |

```powershell
./scripts/check.ps1       # format, lint, test
cargo run -- --settings   # open Settings (any OS)
```

Releases: push a tag such as `v0.2.0`. The release workflow runs the gate, builds the three packages and `SHA256SUMS.txt`, and publishes them.

## What has and hasn't been tested

CI runs the quality gate on Windows, macOS and Linux. **Nothing has been tested by hand, and nothing has been measured.** The binary size and memory use have not been measured yet.

Not yet exercised on a real desktop, on any OS: the snip overlay (including multi-monitor and mixed-DPI setups), text recognition, the global hotkeys, the tray icon, the keystore, and live calls to each provider. Report anything that misbehaves.

## License

MIT. See [LICENSE](LICENSE).
