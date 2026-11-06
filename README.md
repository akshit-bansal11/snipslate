# Snipslate

Snip any part of your screen and **translate it** with your own AI key, or **copy its text**. Like Snipping Tool, but for text.

- **Copy text** (`Ctrl+Alt+C`): drag a box and the text in it is copied to the clipboard. Uses Windows' built-in text recognition, so it works offline and costs nothing.
- **Translate** (`Ctrl+Alt+T`): drag a box and the snip goes to the AI provider you chose. The translation appears in a small popup next to the snip with a **Copy** button.

Snipslate runs in the system tray. Right-click the tray icon for the menu; double-click it to translate.

## Install

Download `Snipslate-<version>.exe` from [Releases](../../releases) and run it. It's one self-contained file: no installer, no .NET runtime needed. Windows 10 (1903) or later, 64-bit.

Windows SmartScreen may warn the first time, because the exe isn't code-signed. Check the file against `SHA256SUMS.txt` from the same release:

```powershell
(Get-FileHash .\Snipslate-0.1.0.exe -Algorithm SHA256).Hash
```

To start it with Windows, put a shortcut to the exe in `shell:startup`.

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

## Privacy

- **Copy text never leaves your PC.**
- **Translate** sends the snipped image (or its text, with *Send text only*) to the provider you configured, and nowhere else. Nothing is sent until you finish a snip.
- API keys are stored in **Windows Credential Manager** (look for `Snipslate` entries), not in a file. Settings live in `%APPDATA%\Snipslate\settings.json` and contain no keys.
- No telemetry, no analytics, no update checks.

## Netflix and other protected video

Protected video (Netflix, Disney+, Prime Video and other DRM players) always captures as a black box. That's the copy protection working as designed, and every screenshot tool sees the same thing. Snipslate doesn't try to get around it. It notices a black snip and tells you instead of sending a wasted, paid request. For video, use the player's own subtitle languages.

## Develop

| Path | What |
| --- | --- |
| `src/Snipslate/` | The app: .NET 10 WinForms, no NuGet dependencies |
| `tests/Snipslate.Tests/` | xUnit tests for hotkey parsing, provider request shapes, reply and error parsing, image checks |
| `scripts/check.ps1` | The quality gate: format, build with warnings as errors, test. `-Ci` verifies formatting without rewriting |
| `.github/workflows/` | `ci.yml` runs the gate and builds the exe on every push; `release.yml` publishes a release for a `v*` tag |

Needs the .NET 10 SDK on Windows.

```powershell
./scripts/check.ps1                                  # format, build, test
dotnet run --project src/Snipslate                    # run it
```

Releases: push a tag such as `v0.1.1`. The workflow runs the gate, builds `Snipslate-0.1.1.exe` and `SHA256SUMS.txt`, and publishes them.

## What has and hasn't been tested

CI compiles the app and runs the unit tests above on every push. **The parts that need a real desktop have not been exercised by an automated test**: the snip overlay (including mixed-DPI multi-monitor setups), Windows text recognition, the global hotkeys, Credential Manager, and live calls to each provider. Report anything that misbehaves.

## License

MIT. See [LICENSE](LICENSE).
