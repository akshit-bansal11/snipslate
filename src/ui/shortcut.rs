//! Keyboard shortcut recorder: shows a shortcut as keycaps and captures a new one by pressing it.
//!
//! ponytail: combos the OS reserves (Win+L, Ctrl+Alt+Del, Cmd+Tab) never reach the app, so they
//! cannot be recorded; the user sees nothing happen and presses another combination.

use std::sync::Arc;

use eframe::egui::{
    self, Color32, Event, EventFilter, FontId, Galley, Key, Rect, Role, Sense, Stroke, StrokeKind,
    TextStyle, Ui, Vec2, WidgetInfo, pos2,
};

use super::icons::{self, Icon};
use super::theme::{self, Palette, radius, size, space};
use crate::hotkey::{Hotkey, held_labels};

/// Minimum control width, so idle and recording look the same size for typical shortcuts.
const MIN_WIDTH: f32 = space::XL * 10.0;
/// Padding inside one keycap.
const CAP_PAD: Vec2 = Vec2::new(space::SM, space::XS);
/// Thickness of the focus ring.
const RING: f32 = 2.0;

/// State kept between frames by whoever owns the widget.
#[derive(Default)]
pub struct Recorder {
    recording: bool,
    error: Option<String>,
}

impl Recorder {
    /// True while the control is waiting for a key press.
    pub fn is_recording(&self) -> bool {
        self.recording
    }
}

fn layout(ui: &Ui, text: &str, font: FontId, color: Color32) -> Arc<Galley> {
    ui.painter().layout_no_wrap(text.to_owned(), font, color)
}

fn cap_galleys(ui: &Ui, labels: &[&str], p: &Palette) -> Vec<Arc<Galley>> {
    let font = theme::strong(size::CAPTION);
    let galley = |label: &&str| layout(ui, label, font.clone(), p.text);
    labels.iter().map(galley).collect()
}

fn caps_width(caps: &[Arc<Galley>]) -> f32 {
    let gaps = caps.len().saturating_sub(1) as f32 * space::XS;
    caps.iter()
        .map(|g| g.size().x + 2.0 * CAP_PAD.x)
        .sum::<f32>()
        + gaps
}

/// Paints keycaps left to right from `x`, vertically centred on `cy`; returns the x after them.
fn paint_caps(ui: &Ui, p: &Palette, caps: &[Arc<Galley>], mut x: f32, cy: f32) -> f32 {
    let painter = ui.painter();
    for galley in caps {
        let cap_size = galley.size() + 2.0 * CAP_PAD;
        let rect = Rect::from_min_size(pos2(x, cy - cap_size.y / 2.0), cap_size);
        painter.rect(
            rect,
            radius::CONTROL,
            p.sunken,
            Stroke::new(1.0, p.control_border),
            StrokeKind::Inside,
        );
        painter.galley(rect.min + CAP_PAD, Arc::clone(galley), p.text);
        x = rect.right() + space::XS;
    }
    x
}

/// True when `value` is not the same shortcut as `default` (compared as parsed hotkeys).
fn differs(value: &str, default: &str) -> bool {
    match (Hotkey::parse(value), Hotkey::parse(default)) {
        (Some(a), Some(b)) => a != b,
        _ => value != default,
    }
}

fn is_modifier_key(key: Key) -> bool {
    matches!(
        key,
        Key::ShiftLeft
            | Key::ShiftRight
            | Key::ControlLeft
            | Key::ControlRight
            | Key::AltLeft
            | Key::AltRight
            | Key::SuperLeft
            | Key::SuperRight
    )
}

/// Draws the recorder for `value` (the stored `Ctrl+Alt+T` text). Returns true when `value`
/// changed, either by recording a new shortcut or by resetting to `default`.
pub fn recorder(
    ui: &mut Ui,
    id_salt: &str,
    rec: &mut Recorder,
    value: &mut String,
    default: &str,
) -> bool {
    let p = theme::palette(ui.ctx());
    let mut changed = false;
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            changed |= control(ui, &p, id_salt, rec, value);
            if differs(value, default)
                && icons::button(ui, Icon::Undo, "", &format!("Reset to {default}")).clicked()
            {
                *value = default.to_owned();
                rec.recording = false;
                rec.error = None;
                changed = true;
            }
        });
        if rec.recording
            && let Some(message) = &rec.error
        {
            ui.label(
                egui::RichText::new(message)
                    .size(size::CAPTION)
                    .color(p.danger),
            );
        }
    });
    changed
}

/// The clickable keycap box. Returns true when a new shortcut was recorded.
fn control(
    ui: &mut Ui,
    p: &Palette,
    id_salt: &str,
    rec: &mut Recorder,
    value: &mut String,
) -> bool {
    let body = TextStyle::Body.resolve(ui.style());
    let parsed = Hotkey::parse(value);
    let was_recording = rec.recording;

    // Content for this frame, measured first so the box can be sized before it is painted.
    let prompt = layout(ui, "Press a shortcut", body.clone(), p.accent_text);
    let hint = layout(
        ui,
        "Esc to cancel",
        FontId::new(size::CAPTION, body.family.clone()),
        p.muted,
    );
    let held = if was_recording {
        held_labels(ui.input(|i| i.modifiers))
    } else {
        Vec::new()
    };
    let idle_caps = parsed.map(|h| h.labels()).unwrap_or_default();
    let caps = cap_galleys(ui, if was_recording { &held } else { &idle_caps }, p);
    let invalid = (!was_recording && parsed.is_none()).then(|| layout(ui, value, body, p.danger));

    let content = if was_recording {
        let held_w = if caps.is_empty() {
            0.0
        } else {
            space::SM + caps_width(&caps)
        };
        prompt.size().x + held_w + space::LG + hint.size().x
    } else {
        invalid
            .as_ref()
            .map_or_else(|| caps_width(&caps), |g| g.size().x)
    };
    let cap_h = layout(ui, "A", theme::strong(size::CAPTION), p.text)
        .size()
        .y
        + 2.0 * CAP_PAD.y;
    let desired = Vec2::new(
        MIN_WIDTH.max(content + 2.0 * space::SM),
        ui.spacing().interact_size.y.max(cap_h + 2.0 * space::XS),
    );

    let id = ui.scope_id().with(("shortcut", id_salt));
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    let mut response = ui.interact(rect, id, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, format!("{id_salt}: {value}")));
    if !was_recording {
        response = response.on_hover_text("Click to change");
    }

    // Paint.
    let painter = ui.painter();
    let fill = if response.hovered() {
        p.hover
    } else {
        p.surface
    };
    painter.rect(
        rect,
        radius::CONTROL,
        fill,
        Stroke::new(1.0, p.control_border),
        StrokeKind::Inside,
    );
    if response.has_focus() || was_recording {
        painter.rect(
            rect,
            radius::CONTROL,
            Color32::TRANSPARENT,
            Stroke::new(RING, p.focus),
            StrokeKind::Inside,
        );
    }
    let cy = rect.center().y;
    let mut x = rect.left() + space::SM;
    if was_recording {
        painter.galley(
            pos2(x, cy - prompt.size().y / 2.0),
            Arc::clone(&prompt),
            p.accent_text,
        );
        x += prompt.size().x + space::SM;
        paint_caps(ui, p, &caps, x, cy);
        let hint_pos = pos2(
            rect.right() - space::SM - hint.size().x,
            cy - hint.size().y / 2.0,
        );
        ui.painter().galley(hint_pos, hint, p.muted);
    } else if let Some(galley) = invalid {
        painter.galley(pos2(x, cy - galley.size().y / 2.0), galley, p.danger);
    } else {
        paint_caps(ui, p, &caps, x, cy);
    }

    // Behaviour.
    let mut changed = false;
    if was_recording {
        ui.ctx().request_repaint();
        ui.ctx().memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                },
            );
        });
        let events = ui.input(|i| i.events.clone());
        for event in events {
            let Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } = event
            else {
                continue;
            };
            if is_modifier_key(key) {
                continue;
            }
            // Keeps Tab, Enter, Space and Esc from reaching focus handling or the window.
            ui.ctx().input_mut(|i| i.consume_key(modifiers, key));
            if key == Key::Escape {
                rec.recording = false;
                rec.error = None;
                break;
            }
            match Hotkey::from_egui(key, modifiers) {
                Ok(hotkey) => {
                    *value = hotkey.to_string();
                    rec.recording = false;
                    rec.error = None;
                    changed = true;
                    break;
                }
                Err(message) => rec.error = Some(message.to_owned()),
            }
        }
        if rec.recording && (response.clicked_elsewhere() || response.lost_focus()) {
            rec.recording = false;
            rec.error = None;
        }
    } else {
        let key_start = response.has_focus()
            && ui.ctx().input_mut(|i| {
                let enter = i.consume_key(egui::Modifiers::NONE, Key::Enter);
                let space = i.consume_key(egui::Modifiers::NONE, Key::Space);
                enter || space
            });
        if response.clicked() || key_start {
            rec.recording = true;
            rec.error = None;
            response.request_focus();
        }
    }
    if rec.recording != was_recording || changed {
        ui.ctx().request_repaint();
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn differs_compares_parsed_hotkeys() {
        assert!(!differs("ctrl+alt+t", "Ctrl+Alt+T"));
        assert!(!differs("Cmd+T", "Win+T"));
        assert!(differs("Ctrl+Alt+Y", "Ctrl+Alt+T"));
        assert!(differs("nonsense", "Ctrl+Alt+T"));
    }
}
