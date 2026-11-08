import type { Block } from "@/lib/docs-types";

export const intro: Block[] = [
  {
    type: "p",
    text: "Snipslate is a tray app for Windows, macOS and Linux. Press a shortcut, the screen freezes, and you drag a box over the text you care about. What happens next depends on the shortcut you pressed.",
  },
  {
    type: "ul",
    items: [
      "**Translate** sends the snip to the AI provider you configured and shows the translation in a small window next to it.",
      "**Copy text** reads the text on your device and puts it on the clipboard. Nothing leaves your computer.",
    ],
  },
  {
    type: "h2",
    id: "quick-start",
    text: "Quick start",
  },
  {
    type: "ol",
    items: [
      "Download the file for your system from [the latest release](https://github.com/akshit-bansal11/snipslate/releases/latest). The [Install](/docs/install) page covers the first-run warnings.",
      "Start Snipslate. It lives in the system tray. Right-click its icon and choose Settings.",
      "Pick a provider, paste your API key and press Test connection. See [Providers](/docs/providers).",
      "Press {{Ctrl+Alt+T}}, drag over some text and release the mouse. On Linux the default is {{Ctrl+Alt+Shift+T}}.",
      "Read the translation, then press Copy.",
    ],
  },
  {
    type: "callout",
    kind: "note",
    text: "Copy text needs no key and no account. Press {{Ctrl+Alt+C}} (Linux: {{Ctrl+Alt+Shift+C}}), drag over text, and it is on your clipboard.",
  },
  {
    type: "h2",
    id: "next",
    text: "Where to go next",
  },
  {
    type: "ul",
    items: [
      "[Translate](/docs/translate): the result window and its keys.",
      "[Copy text](/docs/copy-text): on-device recognition per system.",
      "[Shortcuts](/docs/shortcuts): defaults, recording, and Wayland.",
      "[Privacy](/docs/privacy): exactly what is sent, and where.",
    ],
  },
];

export const install: Block[] = [
  {
    type: "p",
    text: "Each release has one file per system and a SHA256SUMS.txt. Get them from the [latest release](https://github.com/akshit-bansal11/snipslate/releases/latest).",
  },
  {
    type: "h2",
    id: "windows",
    text: "Windows",
  },
  {
    type: "p",
    text: "Needs Windows 10 (1903) or later, 64-bit. Download the .exe and run it. There is no installer. Snipslate appears in the system tray.",
  },
  {
    type: "ul",
    items: [
      "SmartScreen may warn the first time, because the exe is not code-signed. Compare its hash with SHA256SUMS.txt before you run it.",
      "To start it with Windows, put a shortcut to the exe in `shell:startup`.",
      "Settings are in `%APPDATA%\\Snipslate\\settings.json`. API keys are not in that file.",
    ],
  },
  {
    type: "h2",
    id: "macos",
    text: "macOS",
  },
  {
    type: "p",
    text: "The .zip holds a universal Snipslate.app for Intel and Apple silicon. Unzip it and move Snipslate.app to Applications.",
  },
  {
    type: "callout",
    kind: "warning",
    text: "The app is ad-hoc signed, not notarised, so Gatekeeper blocks the first launch. Right-click the app and choose Open, or remove the quarantine flag in Terminal with `xattr -dr com.apple.quarantine Snipslate.app`.",
  },
  {
    type: "ul",
    items: [
      "macOS may ask for Screen Recording permission the first time you snip. Without it, snips capture only the wallpaper.",
      "In shortcuts, Win means Cmd. The tray icon opens its menu on a left click.",
      "Settings are in `~/Library/Application Support/Snipslate/settings.json`.",
    ],
  },
  {
    type: "h2",
    id: "linux",
    text: "Linux",
  },
  {
    type: "p",
    text: "The .tar.gz holds one binary, snipslate, and the license. Extract it and run snipslate. These runtime libraries are needed. The names are for Debian and Ubuntu and are not checked on other distributions.",
  },
  {
    type: "code",
    text: "libgtk-3-0  libayatana-appindicator3-1  libxdo3\nlibdbus-1-3  tesseract-ocr",
  },
  {
    type: "ul",
    items: [
      "API keys are stored in a Secret Service provider such as gnome-keyring. Without one, Snipslate cannot save a key.",
      "GNOME needs the AppIndicator extension to show the tray icon.",
      "On Wayland, screen capture goes through xdg-desktop-portal, and the desktop may ask for permission.",
      "Wayland has no global shortcuts for apps. See [Shortcuts](/docs/shortcuts) for the commands to bind.",
      "Settings are in `$XDG_CONFIG_HOME/snipslate/settings.json`, or `~/.config/snipslate/settings.json`.",
    ],
  },
  {
    type: "h2",
    id: "verify",
    text: "Verify a download",
  },
  {
    type: "p",
    text: "Compare the SHA-256 hash of the file with the line in SHA256SUMS.txt from the same release.",
  },
  {
    type: "table",
    head: ["System", "Command"],
    rows: [
      ["Windows", "`(Get-FileHash .\\<file> -Algorithm SHA256).Hash`"],
      ["macOS", "`shasum -a 256 <file>`"],
      ["Linux", "`sha256sum <file>`"],
    ],
  },
  {
    type: "h2",
    id: "tray",
    text: "The tray menu",
  },
  {
    type: "p",
    text: "Right-click the tray icon for Translate an area, Copy text from an area, Settings and Quit. On Windows a left click translates. On macOS a left click opens the menu. On Linux use the menu or your shortcuts.",
  },
  {
    type: "callout",
    kind: "note",
    text: "Keys saved by the old Windows-only version (.NET) are not read. Enter them again in Settings.",
  },
];

export const providers: Block[] = [
  {
    type: "p",
    text: "Translate needs a provider and an API key that you bring. Open Settings from the tray menu to set them up. Copy text does not use a provider.",
  },
  {
    type: "table",
    head: ["Provider", "Base URL", "Default model"],
    rows: [
      ["Anthropic", "`https://api.anthropic.com/v1`", "`claude-opus-5-5`"],
      ["OpenAI", "`https://api.openai.com/v1`", "`gpt-5-mini`"],
      ["OpenRouter", "`https://openrouter.ai/api/v1`", "`openai/gpt-5-mini`"],
      ["Custom", "`http://localhost:11434/v1`", "none, enter yours"],
    ],
  },
  {
    type: "h2",
    id: "setup",
    text: "Set up a provider",
  },
  {
    type: "ol",
    items: [
      "Open Settings and pick the provider with the Provider switch.",
      "Paste your API key and press Test connection. Snipslate asks the model to translate a short French sentence and shows the reply.",
      "Choose a model, or keep the default.",
    ],
  },
  {
    type: "p",
    text: "If you press the Translate shortcut with no key saved, Snipslate opens Settings with a note saying which key is missing.",
  },
  {
    type: "h2",
    id: "keys",
    text: "Where your key is kept",
  },
  {
    type: "p",
    text: "Keys go to your operating system's keystore, one entry per provider under the service name Snipslate: Windows Credential Manager, the macOS Keychain, or the Linux Secret Service. They are never written to the settings file. Forget key in Settings removes one.",
  },
  {
    type: "h2",
    id: "models",
    text: "Choosing a model",
  },
  {
    type: "p",
    text: "With a key saved, the model picker loads the list of models your key can use. The reload button fetches it again. The model field also accepts a name you type, so you can use any model your account offers.",
  },
  {
    type: "ul",
    items: [
      "Anthropic lists models page by page from its models endpoint.",
      "OpenAI, OpenRouter and custom servers use their /models endpoint. For OpenAI, ids that name embedding, speech, image, moderation or similar non-chat models are hidden.",
      "The list request times out after 20 seconds. A translation request times out after 60 seconds.",
    ],
  },
  {
    type: "h2",
    id: "custom",
    text: "Custom servers",
  },
  {
    type: "p",
    text: "Any OpenAI-compatible server works, such as Ollama, LM Studio or vLLM. Enter its base URL and a model name. The key is optional for a local server.",
  },
  {
    type: "callout",
    kind: "note",
    text: "A base URL must start with https://. Plain http:// is allowed only for localhost, so a key never crosses a network unencrypted. Snipslate also follows no redirects, so a key is never sent to a different address than the one you entered.",
  },
  {
    type: "h2",
    id: "text-only",
    text: "Send text only",
  },
  {
    type: "p",
    text: "By default the image of your snip goes to the model, so the model must be able to read images. For a model that cannot, turn on Send text only. Snipslate reads the text on your device first and sends only the words. This uses the same recognition as [Copy text](/docs/copy-text), so on Linux it is English only.",
  },
];

export const translate: Block[] = [
  {
    type: "p",
    text: "Translate turns text on your screen into the language you chose, using the provider you set up in [Providers](/docs/providers).",
  },
  {
    type: "h2",
    id: "flow",
    text: "How a snip works",
  },
  {
    type: "ol",
    items: [
      "Press the Translate shortcut ({{Ctrl+Alt+T}}, on Linux {{Ctrl+Alt+Shift+T}}) or choose Translate an area in the tray menu.",
      "The screen freezes. Only the monitor under your cursor is captured.",
      "Drag a box over the text and release. A size pill shows the box as you drag.",
      "Nothing is sent until you release. Press Esc or right-click to cancel instead.",
      "A result window opens next to your snip with the translation.",
    ],
  },
  {
    type: "p",
    text: "Pick the target language in Settings under Translate to. The list has 22 languages, from Arabic and Bengali to Vietnamese and Ukrainian.",
  },
  {
    type: "h2",
    id: "result",
    text: "The result window",
  },
  {
    type: "p",
    text: "The header reads Translated to followed by the language. Under it, a caption names the provider and model, and says the text is an AI translation. Below that is the translation, which you can select. The footer has Copy and Close.",
  },
  {
    type: "table",
    head: ["Key", "What it does"],
    rows: [
      ["{{Enter}}", "Copies the translation and closes the window."],
      ["{{Ctrl+C}}", "Copies all of the translation. On macOS use {{Cmd+C}}."],
      ["{{Esc}}", "Closes the window."],
    ],
  },
  {
    type: "p",
    text: "If you have selected part of the text, the usual copy shortcut copies only that selection.",
  },
  {
    type: "h2",
    id: "errors",
    text: "When it fails",
  },
  {
    type: "p",
    text: "A failed request shows the reason in the window. Retry sends the same snip again, and Open settings takes you to your provider and key. See [Troubleshooting](/docs/troubleshooting) for common causes.",
  },
  {
    type: "h2",
    id: "blank",
    text: "Blank or black areas",
  },
  {
    type: "p",
    text: "If the box is almost entirely black, Snipslate stops and says the area looks blank, instead of sending a paid request. Protected video from Netflix and other DRM players always captures as black, because of the copy protection, so there is nothing to translate there. Use the player's own subtitle languages for video.",
  },
  {
    type: "p",
    text: "If the area really holds text on a black background, choose Send anyway.",
  },
  {
    type: "h2",
    id: "scripts",
    text: "Complex scripts",
  },
  {
    type: "callout",
    kind: "warning",
    text: "The result window does not shape text. Arabic, Urdu, Hindi, Bengali and Thai translations appear with letters unjoined and marks misplaced. The Copy button still copies the correct text, so use it for those languages.",
  },
];

export const copyText: Block[] = [
  {
    type: "p",
    text: "Copy text puts the text from an area of your screen on the clipboard. It uses text recognition built into your system, so it works offline, needs no key and costs nothing. The snip never leaves your computer.",
  },
  {
    type: "h2",
    id: "use",
    text: "Using it",
  },
  {
    type: "ol",
    items: [
      "Press the Copy text shortcut ({{Ctrl+Alt+C}}, on Linux {{Ctrl+Alt+Shift+C}}) or choose Copy text from an area in the tray menu.",
      "Drag a box over the text and release. Esc or a right click cancels.",
      "A small toast says how many characters were copied, then closes by itself.",
      "Paste the text anywhere.",
    ],
  },
  {
    type: "h2",
    id: "recognition",
    text: "Recognition on each system",
  },
  {
    type: "table",
    head: ["System", "Engine", "Languages"],
    rows: [
      [
        "Windows",
        "Windows text recognition",
        "The languages installed in your Windows profile.",
      ],
      ["macOS", "Vision", "Built in, no language pack to install."],
      ["Linux", "Tesseract", "English only."],
    ],
  },
  {
    type: "p",
    text: "On Windows, add a language under Settings, Time and language, Language and region. If none is installed, Snipslate tells you. On Linux, install the tesseract-ocr package.",
  },
  {
    type: "h2",
    id: "results",
    text: "What the toast can say",
  },
  {
    type: "ul",
    items: [
      "Copied N characters: the text is on the clipboard.",
      "No text found: try a larger area or a clearer font.",
      "That area is black: see below.",
    ],
  },
  {
    type: "p",
    text: "A black area usually means protected video, which always captures as black. Snipslate cannot read it, and does not try to get around the protection.",
  },
];

export const shortcuts: Block[] = [
  {
    type: "p",
    text: "Two shortcuts start a snip from anywhere: one to translate and one to copy text.",
  },
  {
    type: "h2",
    id: "defaults",
    text: "Defaults",
  },
  {
    type: "table",
    head: ["Action", "Windows and macOS", "Linux"],
    rows: [
      ["Translate an area", "{{Ctrl+Alt+T}}", "{{Ctrl+Alt+Shift+T}}"],
      ["Copy text from an area", "{{Ctrl+Alt+C}}", "{{Ctrl+Alt+Shift+C}}"],
    ],
  },
  {
    type: "h2",
    id: "record",
    text: "Record your own",
  },
  {
    type: "ol",
    items: [
      "Open Settings and find the Shortcuts section.",
      "Click the shortcut box. It shows the keys you are holding.",
      "Press the combination you want. Esc cancels the recording.",
      "Use the reset button next to a shortcut to go back to its default.",
    ],
  },
  {
    type: "h2",
    id: "rules",
    text: "Rules",
  },
  {
    type: "ul",
    items: [
      "A shortcut needs at least one modifier: Ctrl, Alt or Shift, or Cmd on macOS.",
      "The key can be a letter, a digit, F1 to F24, Space, Enter, Tab, Backspace, Insert, Delete, Home, End, Page Up, Page Down, an arrow key, Print Screen or Pause.",
      "The two shortcuts must differ.",
      "The Win or Super key cannot be recorded as a modifier, except Cmd on macOS.",
      "A combination that another app or your system has already claimed cannot be registered. Snipslate tells you, and the tray menu keeps working.",
      "Shortcuts are paused while the Settings window is open.",
    ],
  },
  {
    type: "h2",
    id: "wayland",
    text: "Wayland",
  },
  {
    type: "p",
    text: "Wayland does not let apps register global shortcuts, so Snipslate cannot do it. Bind these commands in your desktop's keyboard shortcut settings instead:",
  },
  {
    type: "code",
    text: "snipslate --translate\nsnipslate --copy",
  },
  {
    type: "p",
    text: "The same two flags work on every system, which also makes them useful for scripts.",
  },
];

export const privacy: Block[] = [
  {
    type: "p",
    text: "Snipslate has no account, no telemetry, no analytics and no update checks. It only talks to the network when you translate or load a model list.",
  },
  {
    type: "table",
    head: ["Action", "What leaves your computer", "Where to"],
    rows: [
      ["Copy text", "Nothing.", "Nowhere."],
      [
        "Translate",
        "The snip image, once you finish a snip.",
        "The provider you configured.",
      ],
      [
        "Translate with Send text only",
        "Only the text read on your device.",
        "The provider you configured.",
      ],
      [
        "Load models, Test connection",
        "Your key, as a request header.",
        "The provider you configured.",
      ],
    ],
  },
  {
    type: "h2",
    id: "network",
    text: "The connection",
  },
  {
    type: "ul",
    items: [
      "Requests go over HTTPS. Plain http:// is accepted only for localhost, such as a local Ollama server.",
      "Redirects are not followed, so a key cannot be forwarded to another address.",
      "Nothing is sent until you finish a snip.",
      "Only the monitor under your cursor is captured.",
    ],
  },
  {
    type: "h2",
    id: "keys",
    text: "Your API keys",
  },
  {
    type: "p",
    text: "Keys are stored in Windows Credential Manager, the macOS Keychain or the Linux Secret Service, under the service name Snipslate. They are not in the settings file or in any other file.",
  },
  {
    type: "h2",
    id: "files",
    text: "Files on disk",
  },
  {
    type: "p",
    text: "Settings live in a settings.json file with your provider, model, language and shortcuts. The location per system is on the [Install](/docs/install) page.",
  },
  {
    type: "h2",
    id: "provider",
    text: "The provider's side",
  },
  {
    type: "p",
    text: "Once a snip reaches your provider, that provider's own terms and retention apply. Check them before you translate anything sensitive, or use a local server and keep it all on your machine.",
  },
];

export const troubleshooting: Block[] = [
  {
    type: "h2",
    id: "shortcut",
    text: "The shortcut does nothing, or says it is in use",
  },
  {
    type: "p",
    text: "Another app or your system has claimed that combination. Record a different one in Settings, as described in [Shortcuts](/docs/shortcuts). The tray menu works in the meantime. On Wayland, shortcuts must be bound in your desktop settings.",
  },
  {
    type: "h2",
    id: "black",
    text: "My snip is black",
  },
  {
    type: "p",
    text: "Protected video from Netflix and other DRM players captures as black. This is the copy protection working. Use the player's subtitle languages instead.",
  },
  {
    type: "p",
    text: "On macOS, a snip that shows only the wallpaper means Screen Recording permission is missing. Allow Snipslate in System Settings, Privacy and Security, Screen Recording, then restart Snipslate.",
  },
  {
    type: "h2",
    id: "keychain",
    text: "No Secret Service on Linux",
  },
  {
    type: "p",
    text: "Snipslate says no Secret Service is available when it cannot reach a keyring. Install and unlock one, for example gnome-keyring or KWallet, then save the key again.",
  },
  {
    type: "h2",
    id: "macos-open",
    text: "macOS will not open the app",
  },
  {
    type: "p",
    text: "The app is ad-hoc signed and not notarised. Right-click Snipslate.app and choose Open, or run this in Terminal:",
  },
  {
    type: "code",
    text: "xattr -dr com.apple.quarantine Snipslate.app",
  },
  {
    type: "h2",
    id: "smartscreen",
    text: "Windows SmartScreen warns about the exe",
  },
  {
    type: "p",
    text: "The exe is not code-signed, so SmartScreen may warn the first time. Compare its SHA-256 hash with SHA256SUMS.txt from the release before you choose to run it.",
  },
  {
    type: "h2",
    id: "script",
    text: "Arabic, Hindi or Thai text looks broken",
  },
  {
    type: "p",
    text: "The result window does not shape text, so letters appear unjoined. The translation itself is correct. Press Copy and paste it into an app that renders the script.",
  },
  {
    type: "h2",
    id: "models",
    text: "The model list does not load",
  },
  {
    type: "ul",
    items: [
      "Check that a key is saved. The list needs one, except on a local server.",
      "Press the reload button, and use Test connection to see the provider's error.",
      "For a custom server, check the base URL. It must start with https://, or http:// for localhost, and must not redirect.",
      "You can still type a model name by hand.",
    ],
  },
  {
    type: "h2",
    id: "no-text",
    text: "Copy text finds nothing",
  },
  {
    type: "p",
    text: "Try a larger area or a clearer font. On Linux, make sure tesseract-ocr is installed. On Windows, make sure a recognition language is installed.",
  },
];
