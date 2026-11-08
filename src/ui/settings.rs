//! The `--settings` window: provider, model, API key, language and hotkeys.

use std::fs::File;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use eframe::egui;

use crate::hotkey::Hotkey;
use crate::settings::{AppSettings, key_get, key_remove, key_set, try_lock};
use crate::translator::{Input, Provider, TranslateJob, is_allowed_base_url, translate};

const LANGUAGES: [&str; 22] = [
    "English",
    "Spanish",
    "French",
    "German",
    "Italian",
    "Portuguese",
    "Dutch",
    "Russian",
    "Ukrainian",
    "Polish",
    "Turkish",
    "Arabic",
    "Hindi",
    "Bengali",
    "Urdu",
    "Chinese (Simplified)",
    "Chinese (Traditional)",
    "Japanese",
    "Korean",
    "Vietnamese",
    "Thai",
    "Indonesian",
];

/// Width of the right-hand column of the form.
const FIELD_WIDTH: f32 = 300.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    BaseUrl,
    Model,
    Language,
    TranslateHotkey,
    CopyHotkey,
}

impl Field {
    fn id(self) -> egui::Id {
        egui::Id::unique(match self {
            Field::BaseUrl => "base_url",
            Field::Model => "model",
            Field::Language => "language",
            Field::TranslateHotkey => "translate_hotkey",
            Field::CopyHotkey => "copy_hotkey",
        })
    }
}

/// The first invalid field and its message, or `None` when the form is valid.
fn find_problem(
    provider: Provider,
    base_url: &str,
    model: &str,
    language: &str,
    translate_hotkey: &str,
    copy_hotkey: &str,
) -> Option<(Field, String)> {
    let Some(translate_key) = Hotkey::parse(translate_hotkey) else {
        return Some((
            Field::TranslateHotkey,
            "The translate hotkey isn't valid. Use a modifier and a key, like Ctrl+Alt+T.".into(),
        ));
    };
    let Some(copy_key) = Hotkey::parse(copy_hotkey) else {
        return Some((
            Field::CopyHotkey,
            "The copy-text hotkey isn't valid. Use a modifier and a key, like Ctrl+Alt+C.".into(),
        ));
    };
    if translate_key == copy_key {
        return Some((
            Field::CopyHotkey,
            "The two hotkeys must be different.".into(),
        ));
    }
    if provider == Provider::Custom && !is_allowed_base_url(base_url.trim()) {
        return Some((
            Field::BaseUrl,
            "The base URL must start with https:// (http:// is allowed only for localhost).".into(),
        ));
    }
    if model.trim().is_empty() {
        return Some((Field::Model, "Enter a model name.".into()));
    }
    if language.trim().is_empty() {
        return Some((
            Field::Language,
            "Choose a language to translate into.".into(),
        ));
    }
    None
}

enum TestState {
    Idle,
    Running(Receiver<Result<String, String>>),
    Done(Result<String, String>),
}

struct SettingsApp {
    settings: AppSettings,
    hint: Option<String>,
    /// Held for the process lifetime so a second settings window exits at once.
    _lock: Option<File>,
    provider: Provider,
    custom_url: String,
    base_url: String,
    model: String,
    api_key: String,
    key_saved: bool,
    key_error: Option<String>,
    language: String,
    text_only: bool,
    translate_hotkey: String,
    copy_hotkey: String,
    message: String,
    focus: Option<Field>,
    test: TestState,
}

impl SettingsApp {
    fn new(settings: AppSettings, hint: Option<&str>, lock: Option<File>) -> Self {
        let mut app = Self {
            hint: hint.map(str::to_string),
            _lock: lock,
            provider: settings.provider,
            custom_url: settings.custom_base_url.clone(),
            base_url: String::new(),
            model: settings.model.clone(),
            api_key: String::new(),
            key_saved: false,
            key_error: None,
            language: settings.target_language.clone(),
            text_only: settings.text_only,
            translate_hotkey: settings.translate_hotkey.clone(),
            copy_hotkey: settings.copy_hotkey.clone(),
            message: String::new(),
            focus: None,
            test: TestState::Idle,
            settings,
        };
        app.show_provider();
        app
    }

    /// Refreshes the base URL box and the saved-key flag. The keychain is read here
    /// only (provider switch, forget, save), never per frame.
    fn show_provider(&mut self) {
        self.base_url = if self.provider == Provider::Custom {
            self.custom_url.clone()
        } else {
            self.provider.default_base_url().to_string()
        };
        match key_get(self.provider) {
            Ok(key) => {
                self.key_saved = key.is_some();
                self.key_error = None;
            }
            Err(e) => {
                self.key_saved = false;
                self.key_error = Some(e);
            }
        }
    }

    fn switch_provider(&mut self, next: Provider) {
        if self.provider == Provider::Custom {
            self.custom_url = self.base_url.trim().to_string();
        }
        let model = self.model.trim();
        if model.is_empty() || model == self.provider.default_model() {
            self.model = next.default_model().to_string();
        }
        self.api_key.clear();
        self.provider = next;
        self.test = TestState::Idle;
        self.show_provider();
    }

    fn key_state_text(&self) -> String {
        let name = self.provider.display_name();
        if let Some(e) = &self.key_error {
            e.clone()
        } else if self.key_saved {
            format!("A key for {name} is saved. Leave the box empty to keep it.")
        } else if self.provider == Provider::Custom {
            "A key is optional for a local server such as Ollama or LM Studio.".to_string()
        } else {
            format!("No key saved for {name} yet.")
        }
    }

    fn problem(&self) -> Option<(Field, String)> {
        find_problem(
            self.provider,
            &self.base_url,
            &self.model,
            &self.language,
            &self.translate_hotkey,
            &self.copy_hotkey,
        )
    }

    fn fail(&mut self, field: Field, text: String) {
        self.message = text;
        self.focus = Some(field);
    }

    fn start_test(&mut self, ctx: &egui::Context) {
        if let Some((field, text)) = self.problem() {
            self.fail(field, text);
            return;
        }
        let typed = self.api_key.trim().to_string();
        let key = if typed.is_empty() {
            key_get(self.provider).ok().flatten().unwrap_or_default()
        } else {
            typed
        };
        if key.is_empty() && self.provider != Provider::Custom {
            self.message = "Paste an API key first.".into();
            return;
        }
        self.message.clear();
        let job = TranslateJob {
            provider: self.provider,
            base_url: self.base_url.trim().to_string(),
            model: self.model.trim().to_string(),
            api_key: key,
            target_language: self.language.trim().to_string(),
            input: Input::Text("Bonjour tout le monde".to_string()),
        };
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            // The receiver is gone only if the window closed; nothing to report then.
            let _ = tx.send(translate(&job));
            ctx.request_repaint();
        });
        self.test = TestState::Running(rx);
    }

    fn poll_test(&mut self) {
        let polled = match &self.test {
            TestState::Running(rx) => Some(rx.try_recv()),
            _ => None,
        };
        match polled {
            Some(Ok(result)) => self.test = TestState::Done(result),
            Some(Err(TryRecvError::Disconnected)) => {
                self.test = TestState::Done(Err("The test stopped unexpectedly.".into()));
            }
            Some(Err(TryRecvError::Empty)) | None => {}
        }
    }

    fn forget_key(&mut self) {
        match key_remove(self.provider) {
            Ok(()) => self.message.clear(),
            Err(e) => self.message = e,
        }
        self.show_provider();
    }

    /// Validates and writes everything. Returns true when the window should close.
    fn save(&mut self) -> bool {
        if let Some((field, text)) = self.problem() {
            self.fail(field, text);
            return false;
        }
        let typed = self.api_key.trim().to_string();
        if !typed.is_empty()
            && let Err(e) = key_set(self.provider, &typed)
        {
            self.message = format!("Couldn't save the key to the system keychain: {e}");
            return false;
        }
        self.api_key.clear();

        let canonical = |text: &str| {
            Hotkey::parse(text).map_or_else(|| text.trim().to_string(), |h| h.to_string())
        };
        self.settings.provider = self.provider;
        self.settings.model = self.model.trim().to_string();
        if self.provider == Provider::Custom {
            self.settings.custom_base_url = self.base_url.trim().to_string();
        }
        self.settings.target_language = self.language.trim().to_string();
        self.settings.text_only = self.text_only;
        self.settings.translate_hotkey = canonical(&self.translate_hotkey);
        self.settings.copy_hotkey = canonical(&self.copy_hotkey);
        match self.settings.save() {
            Ok(()) => true,
            Err(e) => {
                self.message = e;
                false
            }
        }
    }
}

/// Adds `label` and a single-line edit labelled by it. Returns true when Enter was
/// pressed inside the field (the caller saves).
fn edit_row(
    ui: &mut egui::Ui,
    label: &str,
    text: &mut String,
    field: Option<Field>,
    editable: bool,
    password_hint: Option<&str>,
) -> bool {
    let name = ui.label(label);
    let mut edit = egui::TextEdit::singleline(text)
        .desired_width(FIELD_WIDTH)
        .interactive(editable);
    if let Some(field) = field {
        edit = edit.id(field.id());
    }
    if let Some(hint) = password_hint {
        edit = edit.password(true).hint_text(hint);
    }
    let response = ui.add(edit);
    let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    let _ = response.labelled_by(name.id);
    ui.end_row();
    enter
}

impl eframe::App for SettingsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_test();
        let mut submit = false;
        let mut cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let mut test_clicked = false;
        let mut forget_clicked = false;
        let mut save_clicked = false;
        let mut chosen = self.provider;
        let error = ui.visuals().error_fg_color;

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(hint) = &self.hint {
                    ui.group(|ui| {
                        ui.label(hint.as_str());
                    });
                    ui.add_space(6.0);
                }
                egui::Grid::new("form")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        let name = ui.label("Provider");
                        let combo = egui::ComboBox::from_id_salt("provider")
                            .selected_text(self.provider.display_name())
                            .width(FIELD_WIDTH)
                            .show_ui(ui, |ui| {
                                for p in Provider::ALL {
                                    ui.selectable_value(&mut chosen, p, p.display_name());
                                }
                            });
                        let _ = combo.response.labelled_by(name.id);
                        ui.end_row();

                        let editable = self.provider == Provider::Custom;
                        submit |= edit_row(
                            ui,
                            "Base URL",
                            &mut self.base_url,
                            Some(Field::BaseUrl),
                            editable,
                            None,
                        );
                        submit |=
                            edit_row(ui, "Model", &mut self.model, Some(Field::Model), true, None);
                        submit |= edit_row(
                            ui,
                            "API key",
                            &mut self.api_key,
                            None,
                            true,
                            Some("Paste a key to save or replace it"),
                        );

                        ui.label("");
                        ui.horizontal(|ui| {
                            let running = matches!(self.test, TestState::Running(_));
                            test_clicked = ui
                                .add_enabled(!running, egui::Button::new("Test"))
                                .clicked();
                            if self.key_saved {
                                forget_clicked = ui.link("Forget saved key").clicked();
                            }
                        });
                        ui.end_row();

                        ui.label("");
                        ui.vertical(|ui| {
                            ui.set_max_width(FIELD_WIDTH);
                            ui.label(self.key_state_text());
                            match &self.test {
                                TestState::Idle => {}
                                TestState::Running(_) => {
                                    ui.label("Testing…");
                                }
                                TestState::Done(Ok(reply)) => {
                                    ui.label(format!("It works. The model replied: {reply}"));
                                }
                                TestState::Done(Err(e)) => {
                                    ui.colored_label(error, e.as_str());
                                }
                            }
                        });
                        ui.end_row();

                        let name = ui.label("Translate to");
                        ui.horizontal(|ui| {
                            let edit = egui::TextEdit::singleline(&mut self.language)
                                .id(Field::Language.id())
                                .desired_width(FIELD_WIDTH - 110.0);
                            let response = ui.add(edit);
                            submit |= response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let _ = response.labelled_by(name.id);
                            let presets = egui::ComboBox::from_id_salt("language_presets")
                                .selected_text("Presets")
                                .width(90.0)
                                .show_ui(ui, |ui| {
                                    for l in LANGUAGES {
                                        ui.selectable_value(&mut self.language, l.to_string(), l);
                                    }
                                });
                            let _ = presets.response.labelled_by(name.id);
                        });
                        ui.end_row();

                        ui.label("");
                        ui.checkbox(
                            &mut self.text_only,
                            "Send text only (for models that can't read images)",
                        );
                        ui.end_row();

                        submit |= edit_row(
                            ui,
                            "Translate hotkey",
                            &mut self.translate_hotkey,
                            Some(Field::TranslateHotkey),
                            true,
                            None,
                        );
                        submit |= edit_row(
                            ui,
                            "Copy-text hotkey",
                            &mut self.copy_hotkey,
                            Some(Field::CopyHotkey),
                            true,
                            None,
                        );
                    });

                if !self.message.is_empty() {
                    ui.add_space(6.0);
                    ui.colored_label(error, self.message.as_str());
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    save_clicked = ui.button("Save").clicked();
                    cancel |= ui.button("Cancel").clicked();
                });
            });
        });

        if chosen != self.provider {
            self.switch_provider(chosen);
        }
        if forget_clicked {
            self.forget_key();
        }
        if test_clicked {
            self.start_test(ui.ctx());
        }
        if let Some(field) = self.focus.take() {
            ui.memory_mut(|m| m.request_focus(field.id()));
        }
        let saved = (save_clicked || submit) && self.save();
        if saved || cancel {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

pub fn run(hint: Option<&str>) -> i32 {
    let lock = match try_lock("settings.lock") {
        Ok(Some(file)) => Some(file),
        Ok(None) => return 0,
        Err(e) => {
            // Without a lock a second window is possible but harmless; carry on.
            eprintln!("snipslate: settings lock unavailable: {e}");
            None
        }
    };
    let app = SettingsApp::new(AppSettings::load(), hint, lock);
    let viewport = egui::ViewportBuilder::default()
        .with_title("Snipslate settings")
        .with_inner_size([470.0, 560.0])
        .with_resizable(false)
        .with_minimize_button(false)
        .with_maximize_button(false);
    let result = eframe::run_native(
        "Snipslate settings",
        super::native_options(viewport),
        Box::new(move |cc| {
            super::theme::install(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    );
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("snipslate: settings window failed: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: &str = "Ctrl+Alt+T";
    const C: &str = "Ctrl+Alt+C";

    fn check(
        provider: Provider,
        url: &str,
        model: &str,
        lang: &str,
        t: &str,
        c: &str,
    ) -> Option<(Field, String)> {
        find_problem(provider, url, model, lang, t, c)
    }

    #[test]
    fn valid_form_has_no_problem() {
        assert_eq!(check(Provider::Anthropic, "", "m", "English", T, C), None);
    }

    #[test]
    fn bad_translate_hotkey() {
        let (f, m) = check(Provider::Anthropic, "", "m", "English", "T", C).expect("problem");
        assert_eq!(f, Field::TranslateHotkey);
        assert!(m.starts_with("The translate hotkey isn't valid"));
    }

    #[test]
    fn bad_copy_hotkey() {
        let (f, m) = check(Provider::Anthropic, "", "m", "English", T, "Ctrl+").expect("problem");
        assert_eq!(f, Field::CopyHotkey);
        assert!(m.starts_with("The copy-text hotkey isn't valid"));
    }

    #[test]
    fn equal_hotkeys_ignoring_case() {
        let got = check(
            Provider::Anthropic,
            "",
            "m",
            "English",
            "Ctrl+Alt+T",
            "ctrl+alt+t",
        );
        assert_eq!(
            got,
            Some((
                Field::CopyHotkey,
                "The two hotkeys must be different.".to_string()
            ))
        );
    }

    #[test]
    fn custom_http_remote_url_rejected() {
        let (f, _) =
            check(Provider::Custom, "http://example.com", "m", "English", T, C).expect("problem");
        assert_eq!(f, Field::BaseUrl);
    }

    #[test]
    fn non_custom_provider_ignores_base_url() {
        assert_eq!(
            check(Provider::OpenAI, "http://example.com", "m", "English", T, C),
            None
        );
    }

    #[test]
    fn empty_model_and_language() {
        let (f, m) = check(Provider::Anthropic, "", "  ", "English", T, C).expect("problem");
        assert_eq!((f, m.as_str()), (Field::Model, "Enter a model name."));
        let (f, _) = check(Provider::Anthropic, "", "m", " ", T, C).expect("problem");
        assert_eq!(f, Field::Language);
    }
}
