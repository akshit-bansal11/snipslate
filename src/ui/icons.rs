//! The one icon set: line icons painted with egui shapes on a unit grid.
//! No emoji and no icon font, so they cost no binary size and follow the text colour.

use std::f32::consts::PI;

use eframe::egui::{
    self, Atom, Button, Color32, IdSalt, Painter, Pos2, Rect, Response, Stroke, StrokeKind, Ui,
    Vec2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Copy,
    Check,
    CheckCircle,
    Close,
    Refresh,
    Undo,
    Eye,
    EyeOff,
    Alert,
    Settings,
}

/// Default icon box inside buttons and rows.
pub const SIZE: f32 = 16.0;

/// Paints `icon` centred in `rect` in `color`.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let side = rect.width().min(rect.height());
    let origin = rect.center() - Vec2::splat(side / 2.0);
    let at = |x: f32, y: f32| origin + Vec2::new(x, y) * side;
    let stroke = Stroke::new((side / SIZE * 1.5).max(1.0), color);
    let poly = |pts: &[(f32, f32)]| {
        painter.line(pts.iter().map(|&(x, y)| at(x, y)).collect(), stroke);
    };
    // Points on a circle of radius `r`; angle 0 is east and grows clockwise on screen.
    let arc = |cx: f32, cy: f32, r: f32, from: f32, to: f32| -> Vec<Pos2> {
        (0..=24)
            .map(|i| {
                let a = from + (to - from) * i as f32 / 24.0;
                at(cx + r * a.cos(), cy + r * a.sin())
            })
            .collect()
    };

    match icon {
        Icon::Copy => {
            let front = Rect::from_min_max(at(0.16, 0.34), at(0.66, 0.84));
            painter.rect_stroke(front, side * 0.08, stroke, StrokeKind::Middle);
            poly(&[
                (0.34, 0.34),
                (0.34, 0.16),
                (0.84, 0.16),
                (0.84, 0.66),
                (0.66, 0.66),
            ]);
        }
        Icon::Check => poly(&[(0.2, 0.52), (0.42, 0.74), (0.82, 0.28)]),
        Icon::CheckCircle => {
            painter.circle_stroke(at(0.5, 0.5), side * 0.4, stroke);
            poly(&[(0.32, 0.52), (0.45, 0.65), (0.69, 0.38)]);
        }
        Icon::Close => {
            poly(&[(0.25, 0.25), (0.75, 0.75)]);
            poly(&[(0.75, 0.25), (0.25, 0.75)]);
        }
        Icon::Refresh | Icon::Undo => {
            // Three-quarter ring; the arrow head points along the direction of travel.
            const R: f32 = 0.32;
            let (from, to, dir) = if icon == Icon::Undo {
                (PI * 1.3, PI * -0.2, -1.0)
            } else {
                (PI * -0.3, PI * 1.2, 1.0)
            };
            painter.line(arc(0.5, 0.5, R, from, to), stroke);
            let tip = at(0.5 + R * to.cos(), 0.5 + R * to.sin());
            let tangent = Vec2::new(-to.sin(), to.cos()) * dir;
            let radial = Vec2::new(to.cos(), to.sin());
            let len = side * 0.2;
            painter.line(
                vec![
                    tip - tangent * len + radial * len * 0.7,
                    tip,
                    tip - tangent * len - radial * len * 0.7,
                ],
                stroke,
            );
        }
        Icon::Eye | Icon::EyeOff => {
            // Two lid arcs that meet at the eye corners (0.07, 0.5) and (0.93, 0.5).
            let mut lid = arc(0.5, 0.86, 0.56, -PI * 0.778, -PI * 0.222);
            lid.extend(arc(0.5, 0.14, 0.56, PI * 0.222, PI * 0.778));
            lid.push(lid[0]);
            painter.line(lid, stroke);
            painter.circle_stroke(at(0.5, 0.5), side * 0.12, stroke);
            if icon == Icon::EyeOff {
                poly(&[(0.18, 0.18), (0.82, 0.82)]);
            }
        }
        Icon::Alert => {
            painter.circle_stroke(at(0.5, 0.5), side * 0.4, stroke);
            poly(&[(0.5, 0.3), (0.5, 0.55)]);
            painter.circle_filled(at(0.5, 0.69), stroke.width * 0.75, color);
        }
        Icon::Settings => {
            // Sliders: three rails with knobs at different positions.
            for (y, knob) in [(0.26, 0.66), (0.5, 0.34), (0.74, 0.58)] {
                poly(&[(0.16, y), (0.84, y)]);
                painter.circle_filled(at(knob, y), side * 0.1, color);
            }
        }
    }
}

/// Allocates an icon-sized box in the layout and paints `icon` in `color`.
pub fn show(ui: &mut Ui, icon: Icon, color: Color32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(SIZE), egui::Sense::hover());
    paint(ui.painter(), rect, icon, color);
    response
}

/// A regular egui button with `icon` before `text`, painted in the button's own text colour
/// for its current state. An empty `text` gives an icon-only button; `name` is then its
/// tooltip and accessible name (one string for both).
pub fn button(ui: &mut Ui, icon: Icon, text: &str, name: &str) -> Response {
    let id = IdSalt::new("snipslate-icon");
    let slot = Atom::custom(id, Vec2::splat(SIZE));
    let button = if text.is_empty() {
        Button::new(slot)
    } else {
        Button::new((slot, text))
    };
    let atom = button.atom_ui(ui);
    let color = ui.style().interact(&atom.response).text_color();
    if let Some(rect) = atom.rect(id) {
        paint(ui.painter(), rect, icon, color);
    }
    if text.is_empty() {
        atom.response.on_hover_text(name)
    } else {
        atom.response
    }
}
