//! Hotkey strings such as `Ctrl+Alt+T`: parsing, canonical formatting and conversion to
//! `global_hotkey::HotKey`. Registration with the OS happens in the tray process.

use std::fmt;

use eframe::egui;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};

/// One key plus its modifiers. `Eq` lets Settings compare two hotkeys for a clash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hotkey {
    pub modifiers: Modifiers,
    pub code: Code,
}

/// Accepted key names with their canonical spelling, used for both parsing and display.
const KEYS: &[(&str, Code)] = &[
    ("A", Code::KeyA),
    ("B", Code::KeyB),
    ("C", Code::KeyC),
    ("D", Code::KeyD),
    ("E", Code::KeyE),
    ("F", Code::KeyF),
    ("G", Code::KeyG),
    ("H", Code::KeyH),
    ("I", Code::KeyI),
    ("J", Code::KeyJ),
    ("K", Code::KeyK),
    ("L", Code::KeyL),
    ("M", Code::KeyM),
    ("N", Code::KeyN),
    ("O", Code::KeyO),
    ("P", Code::KeyP),
    ("Q", Code::KeyQ),
    ("R", Code::KeyR),
    ("S", Code::KeyS),
    ("T", Code::KeyT),
    ("U", Code::KeyU),
    ("V", Code::KeyV),
    ("W", Code::KeyW),
    ("X", Code::KeyX),
    ("Y", Code::KeyY),
    ("Z", Code::KeyZ),
    ("0", Code::Digit0),
    ("1", Code::Digit1),
    ("2", Code::Digit2),
    ("3", Code::Digit3),
    ("4", Code::Digit4),
    ("5", Code::Digit5),
    ("6", Code::Digit6),
    ("7", Code::Digit7),
    ("8", Code::Digit8),
    ("9", Code::Digit9),
    ("F1", Code::F1),
    ("F2", Code::F2),
    ("F3", Code::F3),
    ("F4", Code::F4),
    ("F5", Code::F5),
    ("F6", Code::F6),
    ("F7", Code::F7),
    ("F8", Code::F8),
    ("F9", Code::F9),
    ("F10", Code::F10),
    ("F11", Code::F11),
    ("F12", Code::F12),
    ("F13", Code::F13),
    ("F14", Code::F14),
    ("F15", Code::F15),
    ("F16", Code::F16),
    ("F17", Code::F17),
    ("F18", Code::F18),
    ("F19", Code::F19),
    ("F20", Code::F20),
    ("F21", Code::F21),
    ("F22", Code::F22),
    ("F23", Code::F23),
    ("F24", Code::F24),
    ("Space", Code::Space),
    ("Enter", Code::Enter),
    ("Tab", Code::Tab),
    ("Backspace", Code::Backspace),
    ("Insert", Code::Insert),
    ("Delete", Code::Delete),
    ("Home", Code::Home),
    ("End", Code::End),
    ("PageUp", Code::PageUp),
    ("PageDown", Code::PageDown),
    ("Up", Code::ArrowUp),
    ("Down", Code::ArrowDown),
    ("Left", Code::ArrowLeft),
    ("Right", Code::ArrowRight),
    ("PrintScreen", Code::PrintScreen),
    ("Pause", Code::Pause),
];

impl Hotkey {
    /// Parses `Mod+...+Key`. Modifiers are case-insensitive and each may appear once; the key
    /// must be a single entry of `KEYS`. Returns `None` for anything else.
    pub fn parse(text: &str) -> Option<Hotkey> {
        let parts: Vec<&str> = text
            .split('+')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();
        let (key_name, modifier_names) = parts.split_last()?;
        if modifier_names.is_empty() {
            return None;
        }

        let mut modifiers = Modifiers::empty();
        for name in modifier_names {
            let flag = match name.to_ascii_uppercase().as_str() {
                "CTRL" | "CONTROL" => Modifiers::CONTROL,
                "ALT" | "OPTION" => Modifiers::ALT,
                "SHIFT" => Modifiers::SHIFT,
                "WIN" | "WINDOWS" | "CMD" | "COMMAND" | "SUPER" | "META" => Modifiers::META,
                _ => return None,
            };
            if modifiers.contains(flag) {
                return None;
            }
            modifiers |= flag;
        }

        let code = KEYS
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(key_name))
            .map(|(_, code)| *code)?;
        Some(Hotkey { modifiers, code })
    }

    /// Builds a hotkey from a key press seen by egui. egui reports the Win/Super key only on
    /// macOS (as `mac_cmd`); `command` is ignored because it duplicates `ctrl` elsewhere.
    pub fn from_egui(key: egui::Key, mods: egui::Modifiers) -> Result<Hotkey, &'static str> {
        let modifiers = from_egui_modifiers(mods);
        let Some(code) = KEYS
            .iter()
            .find(|(name, _)| *name == key.name())
            .map(|(_, code)| *code)
        else {
            return Err("That key can't be used in a shortcut.");
        };
        if modifiers.is_empty() {
            return Err(if cfg!(target_os = "macos") {
                "Add Control, Option, Shift or Cmd to the key."
            } else {
                "Add Ctrl, Alt or Shift to the key."
            });
        }
        Ok(Hotkey { modifiers, code })
    }

    /// Keycap labels for the current OS: modifiers first (in OS order), then the key.
    pub fn labels(&self) -> Vec<&'static str> {
        let mut out = modifier_labels(self.modifiers);
        out.extend(
            KEYS.iter()
                .find(|(_, code)| *code == self.code)
                .map(|(name, _)| *name),
        );
        out
    }

    /// The `global_hotkey` form, ready to register.
    pub fn to_hot_key(self) -> HotKey {
        HotKey::new(Some(self.modifiers), self.code)
    }
}

fn from_egui_modifiers(mods: egui::Modifiers) -> Modifiers {
    let mut modifiers = Modifiers::empty();
    for (down, flag) in [
        (mods.ctrl, Modifiers::CONTROL),
        (mods.alt, Modifiers::ALT),
        (mods.shift, Modifiers::SHIFT),
        (mods.mac_cmd, Modifiers::META),
    ] {
        if down {
            modifiers |= flag;
        }
    }
    modifiers
}

fn modifier_labels(modifiers: Modifiers) -> Vec<&'static str> {
    let names: [&'static str; 4] = if cfg!(target_os = "macos") {
        ["Control", "Option", "Shift", "Cmd"]
    } else if cfg!(windows) {
        ["Ctrl", "Alt", "Shift", "Win"]
    } else {
        ["Ctrl", "Alt", "Shift", "Super"]
    };
    let flags = [
        Modifiers::CONTROL,
        Modifiers::ALT,
        Modifiers::SHIFT,
        Modifiers::META,
    ];
    flags
        .into_iter()
        .zip(names)
        .filter(|(flag, _)| modifiers.contains(*flag))
        .map(|(_, name)| name)
        .collect()
}

/// Keycap labels for the modifiers egui reports as held right now.
pub fn held_labels(mods: egui::Modifiers) -> Vec<&'static str> {
    modifier_labels(from_egui_modifiers(mods))
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let modifiers = [
            (Modifiers::CONTROL, "Ctrl"),
            (Modifiers::ALT, "Alt"),
            (Modifiers::SHIFT, "Shift"),
            (Modifiers::META, "Win"),
        ];
        for (flag, name) in modifiers {
            if self.modifiers.contains(flag) {
                write!(f, "{name}+")?;
            }
        }
        match KEYS.iter().find(|(_, code)| *code == self.code) {
            Some((name, _)) => f.write_str(name),
            None => write!(f, "{:?}", self.code),
        }
    }
}

/// False only on Linux under Wayland, where global hotkeys cannot be registered.
pub fn hotkeys_supported() -> bool {
    !(cfg!(target_os = "linux")
        && std::env::var("XDG_SESSION_TYPE").is_ok_and(|session| session == "wayland"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let cases = [
            (
                "Ctrl+Alt+T",
                Modifiers::CONTROL | Modifiers::ALT,
                Code::KeyT,
            ),
            (
                " ctrl + shift + 5 ",
                Modifiers::CONTROL | Modifiers::SHIFT,
                Code::Digit5,
            ),
            ("Win+F1", Modifiers::META, Code::F1),
            (
                "Control+Alt+PrintScreen",
                Modifiers::CONTROL | Modifiers::ALT,
                Code::PrintScreen,
            ),
        ];
        for (text, modifiers, code) in cases {
            assert_eq!(
                Hotkey::parse(text),
                Some(Hotkey { modifiers, code }),
                "{text}"
            );
        }
    }

    #[test]
    fn rejects() {
        let cases = [
            "",
            "T",
            "Ctrl+",
            "Ctrl+Ctrl+T",
            "Ctrl+Alt+Shift",
            "Hyper+T",
            "Ctrl+65",
            "Ctrl+NotAKey",
            "Ctrl+A, B",
        ];
        for text in cases {
            assert_eq!(Hotkey::parse(text), None, "{text}");
        }
    }

    fn ctrl() -> egui::Modifiers {
        egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        }
    }

    #[test]
    fn from_egui_maps_each_modifier() {
        let cases = [
            (ctrl(), Modifiers::CONTROL),
            (egui::Modifiers::ALT, Modifiers::ALT),
            (egui::Modifiers::SHIFT, Modifiers::SHIFT),
            (egui::Modifiers::MAC_CMD, Modifiers::META),
        ];
        for (mods, expected) in cases {
            let hotkey = Hotkey::from_egui(egui::Key::T, mods).expect("valid");
            assert_eq!(hotkey.modifiers, expected);
            assert_eq!(hotkey.code, Code::KeyT);
        }
    }

    #[test]
    fn from_egui_keys_and_errors() {
        let mods = ctrl();
        assert_eq!(
            Hotkey::from_egui(egui::Key::Num5, mods).map(|h| h.to_string()),
            Ok("Ctrl+5".to_owned())
        );
        assert_eq!(
            Hotkey::from_egui(egui::Key::ArrowUp, mods).map(|h| h.code),
            Ok(Code::ArrowUp)
        );
        assert_eq!(
            Hotkey::from_egui(egui::Key::F12, mods).map(|h| h.code),
            Ok(Code::F12)
        );
        assert!(
            Hotkey::from_egui(egui::Key::T, egui::Modifiers::NONE)
                .is_err_and(|m| m.starts_with("Add "))
        );
        assert_eq!(
            Hotkey::from_egui(egui::Key::Colon, mods),
            Err("That key can't be used in a shortcut.")
        );
    }

    #[test]
    fn parses_aliases() {
        let cases = [
            ("Cmd+T", Modifiers::META),
            ("Command+T", Modifiers::META),
            ("Super+T", Modifiers::META),
            ("Meta+T", Modifiers::META),
            ("Option+T", Modifiers::ALT),
        ];
        for (text, modifiers) in cases {
            let parsed = Hotkey::parse(text).expect(text);
            assert_eq!(parsed.modifiers, modifiers, "{text}");
        }
        assert_eq!(
            Hotkey::parse("Super+T").map(|h| h.to_string()),
            Some("Win+T".to_owned())
        );
    }

    #[test]
    fn labels_list_modifiers_then_key() {
        let labels = Hotkey::parse("Shift+Ctrl+T").expect("valid").labels();
        let first = if cfg!(target_os = "macos") {
            "Control"
        } else {
            "Ctrl"
        };
        assert_eq!(labels, [first, "Shift", "T"]);
    }

    #[test]
    fn formats_in_canonical_order() {
        let hotkey = Hotkey::parse("shift+ctrl+7").expect("valid hotkey");
        assert_eq!(hotkey.to_string(), "Ctrl+Shift+7");
        assert_eq!(Hotkey::parse(&hotkey.to_string()), Some(hotkey));
    }
}
