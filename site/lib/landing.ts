import {
  Gauge,
  Keyboard,
  Lock,
  MonitorPlay,
  Server,
  SunMoon,
} from "lucide-react";

export const hero = {
  title: "Snip any text on your screen. Read it in your language.",
  lead: "Snipslate is a tray app for Windows, macOS and Linux. Press a shortcut, drag over text, and read a translation from the AI provider you choose. Or copy the text with on-device recognition.",
  note: "Free and open source · MIT",
};

export const steps = [
  {
    title: "Press the shortcut",
    body: "The default is Ctrl, Alt and T on Windows and macOS. On Linux the default adds Shift. You can record your own in Settings.",
    keys: "Ctrl+Alt+T",
    href: "/docs/shortcuts",
    linkLabel: "Shortcut rules",
  },
  {
    title: "Drag over any text",
    body: "The screen freezes, so moving content holds still. Snipslate captures the monitor under your cursor. Esc or a right click cancels.",
  },
  {
    title: "Read it or copy it",
    body: "The translation opens next to your snip with a Copy button. Press Enter to copy it and close the window.",
  },
];

export const modes = {
  translate: {
    title: "Translate",
    body: "The snip goes to the provider you set up, with your own key. Pick a model that can read images, or turn on Send text only to read the text on your device first and send only the words.",
    points: [
      "22 target languages",
      "Anthropic, OpenAI, OpenRouter or a custom server",
      "Result window with Copy, Retry and Open settings",
    ],
  },
  copy: {
    title: "Copy text",
    body: "Snipslate reads the text with your operating system, offline. It needs no key and no account, and the snip never leaves your computer.",
    points: [
      "Windows text recognition, macOS Vision, Tesseract on Linux",
      "A small toast confirms how many characters were copied",
      "Tells you when an area is black or has no text",
    ],
  },
};

export const features = [
  {
    title: "Bring your own provider",
    body: "Use Anthropic, OpenAI, OpenRouter, or any OpenAI-compatible server such as Ollama or LM Studio. The models your key can use are listed automatically.",
    icon: Server,
  },
  {
    title: "Keys stay in your OS keychain",
    body: "Windows Credential Manager, the macOS Keychain or the Linux Secret Service. Keys are never written to the settings file.",
    icon: Lock,
  },
  {
    title: "Light on your machine",
    body: "The Windows release exe is about 8.4 MB. Idle in the tray it uses about 12 MB of working set on Windows (2.1 MB private) and about 10 MB on macOS. Measured on CI runners.",
    icon: Gauge,
  },
  {
    title: "Shortcuts you record",
    body: "Click the shortcut box and press the keys you want. Esc cancels the recording, and a reset button brings back the default.",
    icon: Keyboard,
  },
  {
    title: "Knows protected video",
    body: "DRM-protected players capture as black. Snipslate notices and tells you, instead of sending a paid request for an empty image.",
    icon: MonitorPlay,
  },
  {
    title: "Light and dark",
    body: "The app and its windows follow your system theme.",
    icon: SunMoon,
  },
];

export const settingsNotes = [
  {
    title: "Provider and model",
    body: "Switch provider, paste a key, and pick a model from the list your key can use. Test connection checks it with a short translation.",
  },
  {
    title: "Translate to",
    body: "Choose from 22 languages. Send text only suits models that cannot read images.",
  },
  {
    title: "Shortcuts",
    body: "Click, press the keys you want, done. Each shortcut has its own reset button.",
  },
];

export const privacyPoints = [
  "No account to create and no telemetry or analytics in the app.",
  "Copy text stays on your device.",
  "Translate sends the snip only to the provider you configured, over HTTPS.",
  "Nothing is sent until you finish a snip.",
  "Keys live in Windows Credential Manager, the macOS Keychain or the Linux Secret Service.",
];

export const downloads = [
  {
    system: "Windows",
    format: ".exe",
    requirement: "Windows 10 (1903) or later, 64-bit. Run the file, there is no installer.",
    note: "SmartScreen may warn the first time, because the exe is not code-signed.",
  },
  {
    system: "macOS",
    format: ".zip",
    requirement: "Universal build for Intel and Apple silicon. Unzip and move Snipslate.app to Applications.",
    note: "The app is ad-hoc signed, so Gatekeeper needs a right-click, then Open. macOS may ask for Screen Recording permission.",
  },
  {
    system: "Linux",
    format: ".tar.gz",
    requirement: "64-bit. Extract the archive and run snipslate.",
    note: "Needs GTK 3, AppIndicator, libxdo, D-Bus and Tesseract at runtime. The install guide lists them.",
  },
];
