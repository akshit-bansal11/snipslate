//! The `--settings` window: provider, model, API key, language and shortcuts.

use std::fs::File;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

use eframe::egui::{
    self, Align, Atom, Button, Color32, ColorImage, Frame, IdSalt, Key, Layout, Margin, Modifiers,
    Popup, PopupCloseBehavior, Response, RichText, ScrollArea, Stroke, TextEdit, TextureHandle,
    TextureOptions, Ui, Vec2, vec2,
};

use super::icons::{self, Icon};
use super::shortcut::{self, Recorder};
use super::theme::{self, Palette, radius, size, space};
use crate::hotkey::{Hotkey, hotkeys_supported};
use crate::settings::{AppSettings, key_get, key_remove, key_set, try_lock};
use crate::translator::{
    Input, ModelInfo, Provider, TranslateJob, is_allowed_base_url, list_models, translate,
};

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

/// Width of the label column inside a card.
const LABEL_WIDTH: f32 = 110.0;
/// Width of an icon-only button, so the field beside it can be sized to fit.
const ICON_BUTTON: f32 = 44.0;
/// Tallest the picker list grows before it scrolls.
const PICKER_HEIGHT: f32 = 260.0;
/// Where the OS keeps API keys, for the "Key saved" line.
const KEY_STORE: &str = if cfg!(windows) {
    "Windows Credential Manager"
} else if cfg!(target_os = "macos") {
    "macOS Keychain"
} else {
    "your Secret Service keyring"
};

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

    /// The label the user sees for this field.
    fn name(self) -> &'static str {
        match self {
            Field::BaseUrl => "Base URL",
            Field::Model => "Model",
            Field::Language => "Translate to",
            Field::TranslateHotkey => "Translate an area",
            Field::CopyHotkey => "Copy text from an area",
        }
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

type Problem = Option<(Field, String)>;
/// A model-list result tagged with the request generation that asked for it.
type ModelsMsg = (u64, Result<Vec<ModelInfo>, String>);

enum TestState {
    Idle,
    Running(Receiver<Result<String, String>>),
    Done(Result<String, String>),
}

/// The model list for the current provider and key.
enum Models {
    Idle,
    Loading,
    Loaded(Vec<ModelInfo>),
    Failed(String),
}

/// What the widgets asked for this frame; applied after they have run.
#[derive(Default)]
struct Actions {
    chosen: Option<Provider>,
    refetch: bool,
    test: bool,
    forget: bool,
    submit: bool,
}

struct SettingsApp {
    settings: AppSettings,
    defaults: AppSettings,
    hint: Option<String>,
    /// Held for the process lifetime so a second settings window exits at once.
    _lock: Option<File>,
    provider: Provider,
    custom_url: String,
    base_url: String,
    model: String,
    api_key: String,
    show_key: bool,
    key_saved: bool,
    key_error: Option<String>,
    language: String,
    text_only: bool,
    translate_hotkey: String,
    copy_hotkey: String,
    translate_rec: Recorder,
    copy_rec: Recorder,
    hotkeys_ok: bool,
    message: String,
    /// Set after a failed save or test, so field errors show next to their fields.
    attempted: bool,
    focus: Option<Field>,
    test: TestState,
    models: Models,
    models_gen: u64,
    models_tx: Sender<ModelsMsg>,
    models_rx: Receiver<ModelsMsg>,
    initial_fetch: bool,
    logo: Option<TextureHandle>,
}

impl SettingsApp {
    fn new(settings: AppSettings, hint: Option<&str>, lock: Option<File>) -> Self {
        let (models_tx, models_rx) = mpsc::channel();
        let mut app = Self {
            hint: hint.map(str::to_string),
            _lock: lock,
            defaults: AppSettings::default(),
            provider: settings.provider,
            custom_url: settings.custom_base_url.clone(),
            base_url: String::new(),
            model: settings.model.clone(),
            api_key: String::new(),
            show_key: false,
            key_saved: false,
            key_error: None,
            language: settings.target_language.clone(),
            text_only: settings.text_only,
            translate_hotkey: settings.translate_hotkey.clone(),
            copy_hotkey: settings.copy_hotkey.clone(),
            translate_rec: Recorder::default(),
            copy_rec: Recorder::default(),
            hotkeys_ok: hotkeys_supported(),
            message: String::new(),
            attempted: false,
            focus: None,
            test: TestState::Idle,
            models: Models::Idle,
            models_gen: 0,
            models_tx,
            models_rx,
            initial_fetch: true,
            logo: None,
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
        // Results still in flight belong to the old provider.
        self.models_gen += 1;
        self.models = Models::Idle;
        self.show_provider();
    }

    fn problem(&self) -> Problem {
        find_problem(
            self.provider,
            &self.base_url,
            &self.model,
            &self.language,
            &self.translate_hotkey,
            &self.copy_hotkey,
        )
    }

    fn fail(&mut self, field: Field) {
        self.attempted = true;
        self.focus = Some(field);
    }

    fn start_test(&mut self, ctx: &egui::Context) {
        if let Some((field, _)) = self.problem() {
            self.fail(field);
            return;
        }
        let key = self.effective_key();
        if key.is_empty() && self.provider != Provider::Custom {
            self.test = TestState::Done(Err("Paste an API key before testing.".into()));
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

    /// The key to use for a request: the typed one, else the saved one (one keychain read), else empty.
    fn effective_key(&self) -> String {
        let typed = self.api_key.trim();
        if !typed.is_empty() {
            typed.to_string()
        } else if self.key_saved {
            key_get(self.provider).ok().flatten().unwrap_or_default()
        } else {
            String::new()
        }
    }

    /// Starts loading the model list in the background. The typed key wins over the saved one.
    fn fetch_models(&mut self, ctx: &egui::Context) {
        self.models_gen += 1;
        let key = self.effective_key();
        let base = self.base_url.trim().to_string();
        if (key.is_empty() && self.provider != Provider::Custom) || base.is_empty() {
            self.models = Models::Idle;
            return;
        }
        self.models = Models::Loading;
        let (generation, provider, tx, ctx) = (
            self.models_gen,
            self.provider,
            self.models_tx.clone(),
            ctx.clone(),
        );
        std::thread::spawn(move || {
            // The receiver is gone only if the window closed; nothing to report then.
            let _ = tx.send((generation, list_models(provider, &base, &key)));
            ctx.request_repaint();
        });
    }

    fn poll_models(&mut self) {
        while let Ok((generation, result)) = self.models_rx.try_recv() {
            if generation == self.models_gen {
                self.models = match result {
                    Ok(models) => Models::Loaded(models),
                    Err(e) => Models::Failed(e),
                };
            }
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
        if let Some((field, _)) = self.problem() {
            self.fail(field);
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

    fn header(&self, ui: &mut Ui, p: &Palette) {
        ui.horizontal(|ui| {
            if let Some(logo) = &self.logo {
                ui.add(egui::Image::from_texture(logo).fit_to_exact_size(Vec2::splat(40.0)));
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = space::XS;
                ui.label(
                    RichText::new("Snipslate")
                        .font(theme::strong(size::TITLE))
                        .color(p.text),
                );
                ui.label(theme::caption(
                    ui.ctx(),
                    "Snip any part of the screen to copy or translate its text.",
                ));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(theme::caption(
                    ui.ctx(),
                    format!("v{}", env!("CARGO_PKG_VERSION")),
                ));
            });
        });
        if let Some(hint) = &self.hint {
            ui.add_space(space::LG);
            Frame::new()
                .fill(p.surface)
                .stroke(Stroke::new(1.0, p.accent))
                .corner_radius(egui::CornerRadius::same(radius::CARD))
                .inner_margin(Margin::same(space::LG as i8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        icons::show(ui, Icon::Alert, p.accent_text);
                        ui.label(hint.as_str());
                    });
                });
        }
        ui.add_space(space::XL);
    }

    fn provider_card(
        &mut self,
        ui: &mut Ui,
        p: &Palette,
        problem: &Problem,
        focus: Option<Field>,
        act: &mut Actions,
    ) {
        let ctx = ui.ctx().clone();
        let items: Vec<PickItem<'_>> = match &self.models {
            Models::Loaded(models) => models
                .iter()
                .map(|m| PickItem {
                    id: &m.id,
                    label: &m.label,
                    badge: (self.provider == Provider::OpenRouter && m.vision == Some(true))
                        .then_some("Images"),
                })
                .collect(),
            _ => Vec::new(),
        };
        let vision_warning = !self.text_only
            && matches!(&self.models, Models::Loaded(models)
                if models.iter().any(|m| m.id == self.model.trim() && m.vision == Some(false)));
        let no_key =
            self.provider != Provider::Custom && !self.key_saved && self.api_key.trim().is_empty();

        section(
            ui,
            &ctx,
            p,
            "AI provider",
            "Where requests go and which model answers them.",
            |ui| {
                let field_w = (ui.available_width() - LABEL_WIDTH - space::MD).max(180.0);
                egui::Grid::new("provider_grid")
                    .num_columns(2)
                    .spacing([space::MD, space::MD])
                    .min_col_width(LABEL_WIDTH)
                    .show(ui, |ui| {
                        ui.label(theme::muted(&ctx, "Provider"));
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = space::XS;
                            for choice in Provider::ALL {
                                let selected = self.provider == choice;
                                let button = Button::selectable(selected, choice.display_name());
                                if ui.add(button).clicked() && !selected {
                                    act.chosen = Some(choice);
                                }
                            }
                        });
                        ui.end_row();

                        let name = ui.label(theme::muted(&ctx, "API key"));
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                let edit = TextEdit::singleline(&mut self.api_key)
                                    .password(!self.show_key)
                                    .hint_text("Paste a key to save or replace it")
                                    .desired_width(field_w - ICON_BUTTON - space::SM);
                                let response = ui.add(edit);
                                let lost = response.lost_focus();
                                act.submit |= lost && ui.input(|i| i.key_pressed(Key::Enter));
                                if lost && !self.api_key.trim().is_empty() {
                                    act.refetch = true;
                                }
                                let _ = response.labelled_by(name.id);
                                let (icon, tip) = if self.show_key {
                                    (Icon::EyeOff, "Hide API key")
                                } else {
                                    (Icon::Eye, "Show API key")
                                };
                                if icons::button(ui, icon, "", tip).clicked() {
                                    self.show_key = !self.show_key;
                                }
                            });
                            if let Some(e) = &self.key_error {
                                status(ui, Icon::Alert, p.danger, e);
                            } else if self.key_saved {
                                ui.horizontal_wrapped(|ui| {
                                    icons::show(ui, Icon::Check, p.success);
                                    ui.label(theme::caption(
                                        &ctx,
                                        format!("Key saved in {KEY_STORE}."),
                                    ));
                                    if ui.link("Forget key").clicked() {
                                        act.forget = true;
                                    }
                                });
                            } else if self.provider == Provider::Custom {
                                ui.label(theme::caption(
                                    &ctx,
                                    "Optional for a local server such as Ollama or LM Studio.",
                                ));
                            } else {
                                ui.label(theme::caption(
                                    &ctx,
                                    "No key saved yet. Paste one to save it.",
                                ));
                            }
                        });
                        ui.end_row();

                        ui.label(theme::muted(&ctx, "Base URL"));
                        ui.vertical(|ui| {
                            if self.provider == Provider::Custom {
                                let edit = TextEdit::singleline(&mut self.base_url)
                                    .id(Field::BaseUrl.id())
                                    .desired_width(field_w);
                                let response = ui.add(edit);
                                let lost = response.lost_focus();
                                act.submit |= lost && ui.input(|i| i.key_pressed(Key::Enter));
                                act.refetch |= lost;
                                field_error(ui, p, problem, Field::BaseUrl);
                            } else {
                                ui.label(theme::caption(
                                    &ctx,
                                    format!("Endpoint: {}", self.provider.default_base_url()),
                                ));
                            }
                        });
                        ui.end_row();

                        ui.label(theme::muted(&ctx, "Model"));
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                picker(
                                    ui,
                                    "model",
                                    &mut self.model,
                                    &items,
                                    "Choose a model",
                                    field_w - ICON_BUTTON - space::SM,
                                    focus == Some(Field::Model),
                                );
                                if icons::button(ui, Icon::Refresh, "", "Reload models").clicked() {
                                    act.refetch = true;
                                }
                            });
                            match &self.models {
                                Models::Idle if no_key => {
                                    ui.label(theme::caption(
                                        &ctx,
                                        "Add an API key to load the model list.",
                                    ));
                                }
                                Models::Idle => {}
                                Models::Loading => busy(ui, &ctx, "Loading models\u{2026}"),
                                Models::Loaded(models) if models.is_empty() => {
                                    ui.label(theme::caption(
                                        &ctx,
                                        "No models returned. Type a model name.",
                                    ));
                                }
                                Models::Loaded(models) => {
                                    let n = models.len();
                                    let noun = if n == 1 { "model" } else { "models" };
                                    ui.label(theme::caption(&ctx, format!("{n} {noun} available")));
                                }
                                Models::Failed(e) => {
                                    let first = e.lines().next().unwrap_or_default();
                                    status(
                                        ui,
                                        Icon::Alert,
                                        p.danger,
                                        &format!("Couldn't load models: {first}"),
                                    );
                                }
                            }
                            if vision_warning {
                                status(
                                    ui,
                                    Icon::Alert,
                                    p.danger,
                                    "This model can't read images. Turn on Send text only.",
                                );
                            }
                            field_error(ui, p, problem, Field::Model);
                        });
                        ui.end_row();

                        ui.label(theme::muted(&ctx, "Connection"));
                        ui.vertical(|ui| {
                            let running = matches!(self.test, TestState::Running(_));
                            if ui
                                .add_enabled(!running, Button::new("Test connection"))
                                .clicked()
                            {
                                act.test = true;
                            }
                            match &self.test {
                                TestState::Idle => {}
                                TestState::Running(_) => busy(ui, &ctx, "Testing\u{2026}"),
                                TestState::Done(Ok(reply)) => status(
                                    ui,
                                    Icon::Check,
                                    p.success,
                                    &format!("Works. The model replied: {reply}"),
                                ),
                                TestState::Done(Err(e)) => status(ui, Icon::Alert, p.danger, e),
                            }
                        });
                        ui.end_row();
                    });
            },
        );
    }

    fn translation_card(
        &mut self,
        ui: &mut Ui,
        p: &Palette,
        problem: &Problem,
        focus: Option<Field>,
    ) {
        let ctx = ui.ctx().clone();
        let items: Vec<PickItem<'_>> = LANGUAGES
            .iter()
            .map(|l| PickItem {
                id: l,
                label: l,
                badge: None,
            })
            .collect();
        section(
            ui,
            &ctx,
            p,
            "Translation",
            "The language snipped text is translated into.",
            |ui| {
                let field_w = (ui.available_width() - LABEL_WIDTH - space::MD).max(180.0);
                egui::Grid::new("translation_grid")
                    .num_columns(2)
                    .spacing([space::MD, space::MD])
                    .min_col_width(LABEL_WIDTH)
                    .show(ui, |ui| {
                        ui.label(theme::muted(&ctx, "Translate to"));
                        ui.vertical(|ui| {
                            picker(
                                ui,
                                "language",
                                &mut self.language,
                                &items,
                                "Choose a language",
                                field_w,
                                focus == Some(Field::Language),
                            );
                            field_error(ui, p, problem, Field::Language);
                        });
                        ui.end_row();

                        ui.label("");
                        ui.vertical(|ui| {
                            ui.checkbox(&mut self.text_only, "Send text only");
                            ui.label(theme::caption(
                                &ctx,
                                "Reads the text on this device first. Use it for models that can't read images.",
                            ));
                        });
                        ui.end_row();
                    });
            },
        );
    }

    fn shortcuts_card(&mut self, ui: &mut Ui, p: &Palette, problem: &Problem) {
        let ctx = ui.ctx().clone();
        section(
            ui,
            &ctx,
            p,
            "Shortcuts",
            "Keys that start a snip from anywhere.",
            |ui| {
                egui::Grid::new("shortcuts_grid")
                    .num_columns(2)
                    .spacing([space::MD, space::MD])
                    .min_col_width(LABEL_WIDTH)
                    .show(ui, |ui| {
                        ui.add_enabled_ui(self.hotkeys_ok, |ui| {
                            let rows = [
                                (
                                    Field::TranslateHotkey,
                                    &mut self.translate_rec,
                                    &mut self.translate_hotkey,
                                    self.defaults.translate_hotkey.as_str(),
                                ),
                                (
                                    Field::CopyHotkey,
                                    &mut self.copy_rec,
                                    &mut self.copy_hotkey,
                                    self.defaults.copy_hotkey.as_str(),
                                ),
                            ];
                            for (field, rec, value, default) in rows {
                                ui.label(theme::muted(&ctx, field.name()));
                                ui.vertical(|ui| {
                                    shortcut::recorder(ui, field.name(), rec, value, default);
                                    field_error(ui, p, problem, field);
                                });
                                ui.end_row();
                            }
                        });
                    });
                if !self.hotkeys_ok {
                    ui.label(theme::caption(
                        &ctx,
                        "Wayland doesn't let apps register shortcuts. Bind `snipslate --translate` and `snipslate --copy` in your desktop's keyboard settings.",
                    ));
                }
            },
        );
    }
}

/// One entry in a [`picker`].
struct PickItem<'a> {
    id: &'a str,
    label: &'a str,
    badge: Option<&'a str>,
}

/// A button showing the current choice that opens a searchable list. Typing a value that is
/// not in `items` offers it as `Use "..."`, so an empty or failed list never blocks the user.
///
/// ponytail: every matching row is built each frame (fine for a few hundred models) and the
/// list has no arrow-key navigation; Tab and Enter work. Upgrade: `ScrollArea::show_rows`.
fn picker(
    ui: &mut Ui,
    id_salt: &str,
    value: &mut String,
    items: &[PickItem<'_>],
    placeholder: &str,
    width: f32,
    focus: bool,
) -> Response {
    let ctx = ui.ctx().clone();
    let p = theme::palette(&ctx);
    let current = value.trim();
    let shown = items
        .iter()
        .find(|i| i.id == current)
        .map_or(current, |i| i.label);
    let text = if shown.is_empty() {
        RichText::new(placeholder).color(p.muted)
    } else {
        RichText::new(shown)
    };

    let chevron = IdSalt::new(("snipslate-picker", id_salt));
    let atom = Button::new(text)
        .right_text(Atom::custom(chevron, Vec2::splat(12.0)))
        .truncate()
        .min_size(vec2(width, 0.0))
        .atom_ui(ui);
    let color = ui.style().interact(&atom.response).text_color();
    if let Some(rect) = atom.rect(chevron) {
        let c = rect.center();
        let stroke = Stroke::new(1.5, color);
        ui.painter()
            .line_segment([c + vec2(-4.0, -2.0), c + vec2(0.0, 2.0)], stroke);
        ui.painter()
            .line_segment([c + vec2(0.0, 2.0), c + vec2(4.0, -2.0)], stroke);
    }
    let response = atom.response;
    if focus {
        response.request_focus();
    }

    let popup_id = Popup::default_response_id(&response);
    let query_id = popup_id.with("query");
    let edit_id = popup_id.with("search");
    if response.clicked() && !Popup::is_id_open(&ctx, popup_id) {
        ctx.data_mut(|d| d.insert_temp(query_id, String::new()));
        ctx.memory_mut(|m| m.request_focus(edit_id));
    }

    let mut picked: Option<String> = None;
    Popup::from_toggle_button_response(&response)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .width(width)
        .show(|ui| {
            let mut query: String = ctx.data_mut(|d| d.get_temp(query_id)).unwrap_or_default();
            let search = ui.add(
                TextEdit::singleline(&mut query)
                    .id(edit_id)
                    .hint_text("Search or type a name")
                    .desired_width(f32::INFINITY),
            );
            let typed = query.trim().to_string();
            ctx.data_mut(|d| d.insert_temp(query_id, query));

            let needle = typed.to_lowercase();
            let rows: Vec<&PickItem<'_>> = items
                .iter()
                .filter(|i| {
                    needle.is_empty()
                        || i.label.to_lowercase().contains(&needle)
                        || i.id.to_lowercase().contains(&needle)
                })
                .collect();
            let offer_typed = !typed.is_empty() && !items.iter().any(|i| i.id == typed);

            if search.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                if offer_typed {
                    picked = Some(typed.clone());
                } else if let Some(first) = rows.first() {
                    picked = Some(first.id.to_string());
                }
            }

            ScrollArea::vertical()
                .max_height(PICKER_HEIGHT)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
                        if offer_typed && ui.button(format!("Use \"{typed}\"")).clicked() {
                            picked = Some(typed.clone());
                        }
                        for item in &rows {
                            let mut row =
                                Button::selectable(item.id == current, item.label).truncate();
                            let mut side = String::new();
                            if item.id != item.label {
                                side.push_str(item.id);
                            }
                            if let Some(badge) = item.badge {
                                if !side.is_empty() {
                                    side.push_str("  ");
                                }
                                side.push_str(badge);
                            }
                            if !side.is_empty() {
                                row = row.right_text(theme::caption(&ctx, side));
                            }
                            if ui.add(row).clicked() {
                                picked = Some(item.id.to_string());
                            }
                        }
                        if rows.is_empty() && !offer_typed {
                            ui.label(theme::caption(&ctx, "No matches."));
                        }
                    });
                });
        });
    if let Some(choice) = picked {
        *value = choice;
        Popup::close_id(&ctx, popup_id);
    }
    response
}

/// A titled section: heading, one-line caption, then `add` inside a card.
fn section(
    ui: &mut Ui,
    ctx: &egui::Context,
    p: &Palette,
    title: &str,
    caption: &str,
    add: impl FnOnce(&mut Ui),
) {
    ui.label(
        RichText::new(title)
            .font(theme::strong(size::BODY))
            .color(p.text),
    );
    ui.label(theme::caption(ctx, caption));
    ui.add_space(space::XS);
    theme::card(p).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui);
    });
    ui.add_space(space::XL);
}

/// An icon and a caption in the same colour, so status never rests on colour alone.
fn status(ui: &mut Ui, icon: Icon, color: Color32, text: &str) {
    ui.horizontal_wrapped(|ui| {
        icons::show(ui, icon, color);
        ui.label(RichText::new(text).size(size::CAPTION).color(color));
    });
}

fn busy(ui: &mut Ui, ctx: &egui::Context, text: &str) {
    ui.horizontal(|ui| {
        ui.add(egui::Spinner::new().size(size::CAPTION));
        ui.label(theme::caption(ctx, text));
    });
}

/// Shows the problem under `field` when it is that field's.
fn field_error(ui: &mut Ui, p: &Palette, problem: &Problem, field: Field) {
    if let Some((f, text)) = problem
        && *f == field
    {
        status(ui, Icon::Alert, p.danger, text);
    }
}

impl eframe::App for SettingsApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let p = theme::palette(&ctx);
        self.poll_test();
        self.poll_models();
        if std::mem::take(&mut self.initial_fetch)
            && (self.key_saved || self.provider == Provider::Custom)
        {
            self.fetch_models(&ctx);
        }

        // Esc belongs to a recorder or an open list first; read before the widgets run.
        let busy_at_start = self.translate_rec.is_recording()
            || self.copy_rec.is_recording()
            || Popup::is_any_open(&ctx);
        let focus = self.focus.take();
        let problem = if self.attempted { self.problem() } else { None };

        let mut act = Actions::default();
        let mut save_clicked = false;
        let mut cancel = false;

        egui::Panel::bottom("footer")
            .frame(
                Frame::new()
                    .fill(p.surface)
                    .inner_margin(Margin::same(space::MD as i8)),
            )
            .show(ui, |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    save_clicked =
                        theme::primary_scope(ui, |ui| ui.add(Button::new("Save")).clicked());
                    cancel |= ui.button("Cancel").clicked();
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        let summary = match &problem {
                            Some((field, _)) => {
                                Some(format!("Can't save yet. Check {}.", field.name()))
                            }
                            None if !self.message.is_empty() => Some(self.message.clone()),
                            None => None,
                        };
                        if let Some(text) = summary {
                            status(ui, Icon::Alert, p.danger, &text);
                        }
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(Frame::new().fill(p.bg))
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        Frame::new()
                            .inner_margin(Margin::same(space::LG as i8))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                self.header(ui, &p);
                                self.provider_card(ui, &p, &problem, focus, &mut act);
                                self.translation_card(ui, &p, &problem, focus);
                                self.shortcuts_card(ui, &p, &problem);
                            });
                    });
            });

        if let Some(next) = act.chosen {
            self.switch_provider(next);
            act.refetch = true;
        }
        if act.forget {
            self.forget_key();
            act.refetch = true;
        }
        if act.refetch {
            self.fetch_models(&ctx);
        }
        if act.test {
            self.start_test(&ctx);
        }
        if focus == Some(Field::BaseUrl) {
            ui.memory_mut(|m| m.request_focus(Field::BaseUrl.id()));
        }
        if self.focus.is_some() {
            ctx.request_repaint();
        }

        let ctrl_s = ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::S));
        let escape = !busy_at_start && ui.input(|i| i.key_pressed(Key::Escape));
        let saved = (save_clicked || act.submit || ctrl_s) && self.save();
        if saved || cancel || escape {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
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
    let mut app = SettingsApp::new(AppSettings::load(), hint, lock);
    let viewport = egui::ViewportBuilder::default()
        .with_title("Snipslate settings")
        .with_inner_size([560.0, 720.0])
        .with_min_inner_size([480.0, 560.0])
        .with_minimize_button(false)
        .with_maximize_button(false);
    let result = eframe::run_native(
        "Snipslate settings",
        super::native_options(viewport),
        Box::new(move |cc| {
            super::theme::install(&cc.egui_ctx);
            let logo = ColorImage::from_rgba_unmultiplied([64, 64], &crate::brand::logo_rgba(64));
            app.logo = Some(
                cc.egui_ctx
                    .load_texture("logo", logo, TextureOptions::LINEAR),
            );
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
