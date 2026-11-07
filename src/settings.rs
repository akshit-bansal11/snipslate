use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::translator::Provider;

const APP_DIR: &str = "Snipslate";
const SETTINGS_FILE: &str = "settings.json";
const KEY_SERVICE: &str = "Snipslate";

#[cfg(not(target_os = "linux"))]
const DEFAULT_TRANSLATE_HOTKEY: &str = "Ctrl+Alt+T";
#[cfg(not(target_os = "linux"))]
const DEFAULT_COPY_HOTKEY: &str = "Ctrl+Alt+C";
#[cfg(target_os = "linux")]
const DEFAULT_TRANSLATE_HOTKEY: &str = "Ctrl+Alt+Shift+T";
#[cfg(target_os = "linux")]
const DEFAULT_COPY_HOTKEY: &str = "Ctrl+Alt+Shift+C";

/// Directory holding settings.json and lock files. Env vars only, no `dirs` crate.
/// ponytail: a missing HOME/APPDATA falls back to the working directory; upgrade to a
/// platform lookup crate only if that ever happens in practice.
pub fn config_dir() -> PathBuf {
    let fallback = || std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let home = || std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(fallback);
    if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(fallback)
            .join(APP_DIR)
    } else if cfg!(target_os = "macos") {
        home().join("Library/Application Support").join(APP_DIR)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".config"))
            .join("snipslate")
    }
}

/// Opens (creating) `config_dir()/name` and takes an exclusive lock on it.
/// `Ok(None)` when another process holds the lock; keep the File alive to hold it.
pub fn try_lock(name: &str) -> io::Result<Option<File>> {
    fs::create_dir_all(config_dir())?;
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(config_dir().join(name))?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(e)) => Err(e),
    }
}

/// Everything except API keys. Stored as JSON; field names match the C# app's settings.json.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct AppSettings {
    pub provider: Provider,
    pub model: String,
    pub custom_base_url: String,
    pub target_language: String,
    pub text_only: bool,
    pub translate_hotkey: String,
    pub copy_hotkey: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            provider: Provider::Anthropic,
            model: Provider::Anthropic.default_model().to_string(),
            custom_base_url: Provider::Custom.default_base_url().to_string(),
            target_language: "English".to_string(),
            text_only: false,
            translate_hotkey: DEFAULT_TRANSLATE_HOTKEY.to_string(),
            copy_hotkey: DEFAULT_COPY_HOTKEY.to_string(),
        }
    }
}

impl AppSettings {
    pub fn resolve_base_url(&self) -> String {
        match self.provider {
            Provider::Custom => self.custom_base_url.clone(),
            other => other.default_base_url().to_string(),
        }
    }

    /// A missing, unreadable or invalid file gives the defaults. The file holds no keys, so nothing is lost.
    pub fn load() -> AppSettings {
        fs::read_to_string(config_dir().join(SETTINGS_FILE))
            .ok()
            .and_then(|text| Self::from_json(&text))
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let save_err = |e: io::Error| format!("Couldn't save settings: {e}");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Couldn't save settings: {e}"))?;
        fs::create_dir_all(config_dir()).map_err(save_err)?;
        let path = config_dir().join(SETTINGS_FILE);
        let temp = config_dir().join(format!("{SETTINGS_FILE}.tmp"));
        fs::write(&temp, json).map_err(save_err)?;
        fs::rename(&temp, &path).map_err(save_err)
    }

    fn from_json(text: &str) -> Option<AppSettings> {
        serde_json::from_str(text).ok()
    }
}

fn key_entry(provider: Provider) -> Result<keyring::v1::Entry, String> {
    keyring::v1::Entry::new(KEY_SERVICE, provider.name()).map_err(|e| keyring_message(&e))
}

/// Fixed, readable text only. Never includes the key or the library's error payload.
fn keyring_message(e: &keyring::v1::Error) -> String {
    use keyring::v1::Error;
    match e {
        Error::NoStorageAccess { .. } | Error::PlatformFailure { .. } | Error::NoDefaultStore { .. }
            if cfg!(target_os = "linux") =>
        {
            "No Secret Service (gnome-keyring or KWallet) is available to store the key.".to_string()
        }
        _ => "Couldn't access the OS keychain to read or store the API key.".to_string(),
    }
}

/// `Ok(None)` when no key is stored or the stored key is empty.
pub fn key_get(p: Provider) -> Result<Option<String>, String> {
    match key_entry(p)?.get_password() {
        Ok(key) if key.is_empty() => Ok(None),
        Ok(key) => Ok(Some(key)),
        Err(keyring::v1::Error::NoEntry { .. }) => Ok(None),
        Err(e) => Err(keyring_message(&e)),
    }
}

pub fn key_set(p: Provider, key: &str) -> Result<(), String> {
    key_entry(p)?
        .set_password(key)
        .map_err(|e| keyring_message(&e))
}

pub fn key_remove(p: Provider) -> Result<(), String> {
    match key_entry(p)?.delete_credential() {
        Ok(()) | Err(keyring::v1::Error::NoEntry { .. }) => Ok(()),
        Err(e) => Err(keyring_message(&e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn defaults_match_the_csharp_defaults() {
        let s = AppSettings::default();
        assert_eq!(s.provider, Provider::Anthropic);
        assert_eq!(s.model, "claude-opus-5-5");
        assert_eq!(s.custom_base_url, "http://localhost:11434/v1");
        assert_eq!(s.target_language, "English");
        assert!(!s.text_only);
        assert_eq!(s.translate_hotkey, "Ctrl+Alt+T");
        assert_eq!(s.copy_hotkey, "Ctrl+Alt+C");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_defaults_use_shift_hotkeys() {
        let s = AppSettings::default();
        assert_eq!(s.translate_hotkey, "Ctrl+Alt+Shift+T");
        assert_eq!(s.copy_hotkey, "Ctrl+Alt+Shift+C");
    }

    #[test]
    fn loads_a_file_written_by_the_csharp_app() {
        let json = r#"{
            "Provider": "OpenRouter",
            "Model": "openai/gpt-5-mini",
            "CustomBaseUrl": "http://localhost:11434/v1",
            "TargetLanguage": "French",
            "TextOnly": true,
            "TranslateHotkey": "Ctrl+Alt+F",
            "CopyHotkey": "Ctrl+Alt+G"
        }"#;
        let s = AppSettings::from_json(json).expect("valid C# settings");
        assert_eq!(s.provider, Provider::OpenRouter);
        assert_eq!(s.model, "openai/gpt-5-mini");
        assert_eq!(s.target_language, "French");
        assert!(s.text_only);
        assert_eq!(s.translate_hotkey, "Ctrl+Alt+F");
        assert_eq!(s.copy_hotkey, "Ctrl+Alt+G");
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let s = AppSettings::from_json(r#"{"Provider": "OpenAI"}"#).expect("valid JSON");
        assert_eq!(s.provider, Provider::OpenAI);
        assert_eq!(s.target_language, "English");
        assert_eq!(s.translate_hotkey, AppSettings::default().translate_hotkey);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let s = AppSettings::from_json(r#"{"Provider": "Custom", "Future": 1}"#);
        assert_eq!(s.map(|s| s.provider), Some(Provider::Custom));
    }

    #[test]
    fn invalid_json_gives_none() {
        assert_eq!(AppSettings::from_json("{not json"), None);
    }

    #[test]
    fn round_trip_keeps_pascal_case_names() {
        let json = serde_json::to_string(&AppSettings::default()).expect("serializes");
        assert!(json.contains("\"CustomBaseUrl\""));
        assert!(json.contains("\"TextOnly\""));
        assert!(json.contains("\"Provider\":\"Anthropic\""));
        assert_eq!(AppSettings::from_json(&json), Some(AppSettings::default()));
    }

    #[test]
    fn resolve_base_url_uses_custom_only_for_custom() {
        let mut s = AppSettings::default();
        s.custom_base_url = "https://example.test/v1".to_string();
        assert_eq!(s.resolve_base_url(), Provider::Anthropic.default_base_url());
        s.provider = Provider::Custom;
        assert_eq!(s.resolve_base_url(), "https://example.test/v1");
    }
}
