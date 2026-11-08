//! Design tokens and the egui style built from them. Every window calls [`install`] once;
//! custom painting reads colours from [`palette`], never from literals.
//!
//! Contrast (WCAG 2.2, measured on the hex values below):
//! - text on surface: light 17.9:1, dark 13.4:1; muted text: light 7.6:1, dark 5.7:1
//! - white label on accent `#2563EB`: 5.17:1 (same fill in both themes, so it always holds)
//! - control borders (`#64748B`): 4.76:1 on white, 3.05:1 on `#1E293B`
//! - focus ring: `#2563EB` on white 5.17:1, `#93C5FD` on `#1E293B` 8.6:1
//! - danger text: `#B91C1C` on `#F1F5F9` 5.96:1, `#F87171` on `#1E293B` 5.3:1
//! - success text: `#15803D` on white 5.02:1, `#4ADE80` on `#1E293B` 8.4:1

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Frame, Margin,
    RichText, Shadow, Stroke, TextStyle, Theme, Visuals,
};

/// Raw scale. Only [`Palette::of`] names these; components use the semantic roles.
mod raw {
    use eframe::egui::Color32;

    pub const SLATE_50: Color32 = Color32::from_rgb(0xF8, 0xFA, 0xFC);
    pub const SLATE_100: Color32 = Color32::from_rgb(0xF1, 0xF5, 0xF9);
    pub const SLATE_200: Color32 = Color32::from_rgb(0xE2, 0xE8, 0xF0);
    pub const SLATE_300: Color32 = Color32::from_rgb(0xCB, 0xD5, 0xE1);
    pub const SLATE_400: Color32 = Color32::from_rgb(0x94, 0xA3, 0xB8);
    pub const SLATE_500: Color32 = Color32::from_rgb(0x64, 0x74, 0x8B);
    pub const SLATE_600: Color32 = Color32::from_rgb(0x47, 0x55, 0x69);
    pub const SLATE_700: Color32 = Color32::from_rgb(0x33, 0x41, 0x55);
    pub const SLATE_800: Color32 = Color32::from_rgb(0x1E, 0x29, 0x3B);
    pub const SLATE_900: Color32 = Color32::from_rgb(0x0F, 0x17, 0x2A);
    pub const SLATE_950: Color32 = Color32::from_rgb(0x0B, 0x11, 0x20);
    pub const WHITE: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
    pub const OVERLAY_PILL: Color32 = Color32::from_black_alpha(210);
    pub const BLUE_100: Color32 = Color32::from_rgb(0xDB, 0xEA, 0xFE);
    pub const BLUE_300: Color32 = Color32::from_rgb(0x93, 0xC5, 0xFD);
    pub const BLUE_400: Color32 = Color32::from_rgb(0x60, 0xA5, 0xFA);
    pub const BLUE_600: Color32 = Color32::from_rgb(0x25, 0x63, 0xEB);
    pub const BLUE_700: Color32 = Color32::from_rgb(0x1D, 0x4E, 0xD8);
    pub const BLUE_800: Color32 = Color32::from_rgb(0x1E, 0x40, 0xAF);
    pub const BLUE_900: Color32 = Color32::from_rgb(0x1E, 0x3A, 0x8A);
    pub const RED_400: Color32 = Color32::from_rgb(0xF8, 0x71, 0x71);
    pub const RED_700: Color32 = Color32::from_rgb(0xB9, 0x1C, 0x1C);
    pub const RED_TINT_LIGHT: Color32 = Color32::from_rgb(0xFE, 0xF2, 0xF2);
    pub const RED_TINT_DARK: Color32 = Color32::from_rgb(0x3B, 0x1D, 0x24);
    pub const GREEN_400: Color32 = Color32::from_rgb(0x4A, 0xDE, 0x80);
    pub const GREEN_700: Color32 = Color32::from_rgb(0x15, 0x80, 0x3D);
}

/// Semantic colour roles, defined for both themes.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// Window background.
    pub bg: Color32,
    /// Cards and popups.
    pub surface: Color32,
    /// Text fields and other wells.
    pub sunken: Color32,
    /// Hovered and pressed fill of secondary controls.
    pub hover: Color32,
    pub pressed: Color32,
    /// Decorative card and divider lines.
    pub border: Color32,
    /// Boundary of inputs and secondary buttons (3:1 or better).
    pub control_border: Color32,
    pub text: Color32,
    pub muted: Color32,
    /// Filled primary control.
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_pressed: Color32,
    pub on_accent: Color32,
    /// Accent-coloured text and links on `bg`/`surface`.
    pub accent_text: Color32,
    pub focus: Color32,
    pub selection: Color32,
    pub success: Color32,
    pub danger: Color32,
    /// Background of an error banner.
    pub danger_bg: Color32,
    /// Dimming painted over the frozen screenshot.
    pub scrim: Color32,
    /// Pills painted over the screenshot; the same in both themes because the screenshot is.
    pub overlay_pill: Color32,
    pub overlay_text: Color32,
}

impl Palette {
    pub fn of(theme: Theme) -> Palette {
        use raw::*;
        let shared_accent = (BLUE_600, BLUE_700, BLUE_800, WHITE);
        let (accent, accent_hover, accent_pressed, on_accent) = shared_accent;
        match theme {
            Theme::Light => Palette {
                bg: SLATE_100,
                surface: WHITE,
                sunken: SLATE_50,
                hover: SLATE_200,
                pressed: SLATE_300,
                border: SLATE_200,
                control_border: SLATE_500,
                text: SLATE_900,
                muted: SLATE_600,
                accent,
                accent_hover,
                accent_pressed,
                on_accent,
                accent_text: BLUE_700,
                focus: BLUE_600,
                selection: BLUE_100,
                success: GREEN_700,
                danger: RED_700,
                danger_bg: RED_TINT_LIGHT,
                scrim: Color32::from_black_alpha(115),
                overlay_pill: OVERLAY_PILL,
                overlay_text: WHITE,
            },
            Theme::Dark => Palette {
                bg: SLATE_900,
                surface: SLATE_800,
                sunken: SLATE_950,
                hover: SLATE_700,
                pressed: SLATE_600,
                border: SLATE_700,
                control_border: SLATE_500,
                text: SLATE_100,
                muted: SLATE_400,
                accent,
                accent_hover,
                accent_pressed,
                on_accent,
                accent_text: BLUE_400,
                focus: BLUE_300,
                selection: BLUE_900,
                success: GREEN_400,
                danger: RED_400,
                danger_bg: RED_TINT_DARK,
                scrim: Color32::from_black_alpha(115),
                overlay_pill: OVERLAY_PILL,
                overlay_text: WHITE,
            },
        }
    }
}

/// The palette for the theme the window is showing now.
pub fn palette(ctx: &egui::Context) -> Palette {
    Palette::of(ctx.theme())
}

/// Spacing scale (points). Every margin, padding and gap is one of these.
pub mod space {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
}

/// Radius scale: controls < cards < floating surfaces (toasts, pills).
pub mod radius {
    pub const CONTROL: u8 = 6;
    pub const CARD: u8 = 10;
    pub const SURFACE: u8 = 14;
}

/// Type scale (points). Line height is egui's default for each size.
pub mod size {
    pub const CAPTION: f32 = 12.0;
    pub const BODY: f32 = 14.0;
    pub const READING: f32 = 16.0;
    pub const TITLE: f32 = 18.0;
}

const STRONG: &str = "Strong";
const READING: &str = "Reading";

/// Semibold UI font (falls back to the regular one where the OS has no separate file).
pub fn strong(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(STRONG.into()))
}

/// The larger text style used for translated text.
pub fn reading() -> TextStyle {
    TextStyle::Name(READING.into())
}

/// Secondary text.
pub fn muted(ctx: &egui::Context, text: impl Into<String>) -> RichText {
    RichText::new(text).color(palette(ctx).muted)
}

/// Small secondary text.
pub fn caption(ctx: &egui::Context, text: impl Into<String>) -> RichText {
    RichText::new(text)
        .size(size::CAPTION)
        .color(palette(ctx).muted)
}

/// A card: surface fill, hairline border, card radius, roomy padding.
pub fn card(p: &Palette) -> Frame {
    Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(radius::CARD))
        .inner_margin(Margin::same(space::LG as i8))
}

/// Shadow scale: one elevation step, for popups and windows.
pub fn shadow_overlay() -> Shadow {
    Shadow {
        offset: [0, 6],
        blur: 20,
        spread: 0,
        color: Color32::from_black_alpha(60),
    }
}

/// One system UI font file: path and face index inside a collection.
fn system_font(bold: bool) -> Option<FontData> {
    let (path, index) = if cfg!(target_os = "linux") {
        let pattern = if bold { "sans:weight=semibold" } else { "sans" };
        let out = std::process::Command::new("fc-match")
            .args(["-f", "%{file}\n%{index}", pattern])
            .output()
            .ok()?;
        let text = String::from_utf8(out.stdout).ok()?;
        let mut lines = text.lines();
        let path = PathBuf::from(lines.next()?.trim());
        let index = lines
            .next()
            .and_then(|i| i.trim().parse().ok())
            .unwrap_or(0);
        (path, index)
    } else if cfg!(windows) {
        let dir =
            std::env::var_os("WINDIR").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
        (
            dir.join("Fonts")
                .join(if bold { "seguisb.ttf" } else { "segoeui.ttf" }),
            0,
        )
    } else {
        // ponytail: SF Pro is one variable file and egui renders its default (regular) instance,
        // so macOS has no separate semibold; headings rely on size. Upgrade: variation axes.
        if bold {
            return None;
        }
        (PathBuf::from("/System/Library/Fonts/SFNS.ttf"), 0)
    };
    let bytes = std::fs::read(path).ok()?;
    let mut data = FontData::from_owned(bytes);
    data.index = index;
    Some(data)
}

/// egui's fonts with the OS UI font in front, plus the `Strong` family.
/// A missing system font leaves egui's bundled font in charge; nothing fails.
pub fn base_fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let mut proportional = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    if let Some(regular) = system_font(false) {
        fonts.font_data.insert("ui".into(), Arc::new(regular));
        proportional.insert(0, "ui".into());
    }
    let mut strong_family = proportional.clone();
    if let Some(bold) = system_font(true) {
        fonts.font_data.insert("ui-strong".into(), Arc::new(bold));
        strong_family.insert(0, "ui-strong".into());
    }
    fonts
        .families
        .insert(FontFamily::Proportional, proportional);
    fonts
        .families
        .insert(FontFamily::Name(STRONG.into()), strong_family);
    fonts
}

fn visuals(theme: Theme) -> Visuals {
    let p = Palette::of(theme);
    let mut v = match theme {
        Theme::Light => Visuals::light(),
        Theme::Dark => Visuals::dark(),
    };
    let control = CornerRadius::same(radius::CONTROL);
    v.panel_fill = p.bg;
    v.window_fill = p.surface;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(radius::CARD);
    v.window_shadow = shadow_overlay();
    v.popup_shadow = shadow_overlay();
    v.menu_corner_radius = CornerRadius::same(radius::CARD);
    v.faint_bg_color = p.sunken;
    v.extreme_bg_color = p.sunken;
    v.text_edit_bg_color = Some(p.sunken);
    v.code_bg_color = p.sunken;
    v.hyperlink_color = p.accent_text;
    v.error_fg_color = p.danger;
    v.warn_fg_color = p.danger;
    v.weak_text_color = Some(p.muted);
    v.selection.bg_fill = p.selection;
    // Also the focused text-field outline in egui 0.37.
    v.selection.stroke = Stroke::new(2.0, p.focus);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.surface;
    w.noninteractive.weak_bg_fill = p.surface;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = control;

    w.inactive.bg_fill = p.surface;
    w.inactive.weak_bg_fill = p.surface;
    w.inactive.bg_stroke = Stroke::new(1.0, p.control_border);
    w.inactive.fg_stroke = Stroke::new(1.0, p.text);
    w.inactive.corner_radius = control;

    w.hovered.bg_fill = p.hover;
    w.hovered.weak_bg_fill = p.hover;
    w.hovered.bg_stroke = Stroke::new(1.0, p.control_border);
    w.hovered.fg_stroke = Stroke::new(1.0, p.text);
    w.hovered.corner_radius = control;

    // `active` is also the keyboard-focus state, so its outline is the focus ring.
    w.active.bg_fill = p.pressed;
    w.active.weak_bg_fill = p.pressed;
    w.active.bg_stroke = Stroke::new(2.0, p.focus);
    w.active.fg_stroke = Stroke::new(1.0, p.text);
    w.active.corner_radius = control;

    w.open = w.active;
    // No hover growth: a state change never moves the layout.
    for state in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        state.expansion = 0.0;
    }
    v
}

/// Fonts, text styles, spacing and both themes' visuals. The OS theme picks which one shows.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(base_fonts());
    ctx.all_styles_mut(|style| {
        use egui::FontFamily::{Monospace, Proportional};
        style.text_styles = [
            (TextStyle::Small, FontId::new(size::CAPTION, Proportional)),
            (TextStyle::Body, FontId::new(size::BODY, Proportional)),
            (TextStyle::Button, FontId::new(size::BODY, Proportional)),
            (TextStyle::Heading, strong(size::TITLE)),
            (
                TextStyle::Monospace,
                FontId::new(size::BODY - 1.0, Monospace),
            ),
            (reading(), FontId::new(size::READING, Proportional)),
        ]
        .into();
        let s = &mut style.spacing;
        s.item_spacing = egui::vec2(space::SM, space::SM);
        s.button_padding = egui::vec2(space::MD, space::XS + 2.0);
        s.interact_size.y = 30.0;
        s.window_margin = Margin::same(space::LG as i8);
        s.menu_margin = Margin::same(space::XS as i8);
        s.icon_spacing = space::SM - 2.0;
    });
    ctx.set_visuals_of(Theme::Light, visuals(Theme::Light));
    ctx.set_visuals_of(Theme::Dark, visuals(Theme::Dark));
}

/// Runs `add` with buttons styled as the filled primary action.
pub fn primary_scope<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let p = palette(ui.ctx());
    ui.scope(|ui| {
        let w = &mut ui.visuals_mut().widgets;
        for (state, fill) in [
            (&mut w.inactive, p.accent),
            (&mut w.hovered, p.accent_hover),
            (&mut w.active, p.accent_pressed),
        ] {
            state.weak_bg_fill = fill;
            state.bg_fill = fill;
            state.fg_stroke = Stroke::new(1.0, p.on_accent);
        }
        w.inactive.bg_stroke = Stroke::NONE;
        w.hovered.bg_stroke = Stroke::NONE;
        // Keep the focus ring, offset from the fill by its own colour.
        add(ui)
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG relative luminance.
    fn luminance(c: Color32) -> f64 {
        let lin = |v: u8| {
            let s = f64::from(v) / 255.0;
            if s <= 0.040_45 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
    }

    fn contrast(a: Color32, b: Color32) -> f64 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    #[test]
    fn text_roles_meet_aa_in_both_themes() {
        for theme in [Theme::Light, Theme::Dark] {
            let p = Palette::of(theme);
            for bg in [p.bg, p.surface, p.sunken] {
                assert!(contrast(p.text, bg) >= 4.5, "{theme:?} text");
                assert!(contrast(p.muted, bg) >= 4.5, "{theme:?} muted");
                assert!(contrast(p.danger, bg) >= 4.5, "{theme:?} danger");
                assert!(contrast(p.success, bg) >= 4.5, "{theme:?} success");
                assert!(contrast(p.accent_text, bg) >= 4.5, "{theme:?} accent text");
            }
            assert!(
                contrast(p.danger, p.danger_bg) >= 4.5,
                "{theme:?} danger banner"
            );
            assert!(
                contrast(p.text, p.selection) >= 4.5,
                "{theme:?} selected text"
            );
            assert!(contrast(p.text, p.hover) >= 4.5, "{theme:?} hovered label");
            assert!(
                contrast(p.text, p.pressed) >= 4.5,
                "{theme:?} pressed label"
            );
        }
    }

    #[test]
    fn filled_accent_labels_meet_the_text_bar() {
        for theme in [Theme::Light, Theme::Dark] {
            let p = Palette::of(theme);
            for fill in [p.accent, p.accent_hover, p.accent_pressed] {
                assert!(contrast(p.on_accent, fill) >= 4.5, "{theme:?} accent label");
            }
        }
    }

    #[test]
    fn boundaries_and_focus_meet_the_non_text_bar() {
        for theme in [Theme::Light, Theme::Dark] {
            let p = Palette::of(theme);
            for bg in [p.surface, p.sunken] {
                assert!(contrast(p.control_border, bg) >= 3.0, "{theme:?} border");
                assert!(contrast(p.focus, bg) >= 3.0, "{theme:?} focus");
            }
        }
    }
}
