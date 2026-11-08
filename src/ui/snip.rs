//! Snip child process: freezes one monitor, lets the user drag a box, then shows the OCR / translation
//! result in the same window. One `run_native`; the screenshot is uploaded as a texture exactly once.

use std::fs::File;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Color32, ColorImage, CornerRadius, CursorIcon, FontId, Frame, Id, Key, Layout,
    Margin, Painter, PointerButton, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind,
    TextEdit, TextureHandle, TextureOptions, UiBuilder, Vec2, ViewportBuilder, ViewportCommand,
    WindowLevel, pos2, vec2,
};
use image::RgbaImage;
use mouse_position::mouse_position::Mouse;

use crate::settings::{self, AppSettings};
use crate::translator::{self, Input, Provider, TranslateJob, Translation};
use crate::ui::icons::{self, Icon};
use crate::ui::theme::{self, Palette, radius, size, space};
use crate::{clip, imaging, ocr, ui};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Translate,
    Copy,
}

const MIN_SIDE: u32 = 4;
const GAP: f32 = 8.0;
const AUTO_CLOSE: Duration = Duration::from_millis(1500);
const HINT: &str = "Drag to select an area  \u{b7}  Esc to cancel";
const DRM: &str = "Protected video (Netflix and other DRM players)";
const HAIRLINE: f32 = 1.0;
/// Selection outline and handle outline.
const OUTLINE: f32 = 2.0;
const HANDLE: f32 = 8.0;
const HINT_TOP: f32 = 44.0;
const RESULT_SIZE: Vec2 = vec2(480.0, 420.0);
const RESULT_MIN: Vec2 = vec2(320.0, 200.0);
const TOAST_SIZE: Vec2 = vec2(340.0, 84.0);
const THUMB: Vec2 = vec2(72.0, 48.0);
const SKELETON: [f32; 3] = [1.0, 0.92, 0.6];

/// Maps a selection in window points to a pixel rectangle `(x, y, w, h)` of an image of `size` pixels
/// that covers `screen` (points). `None` when either side is under 4 px.
fn selection_to_pixels(sel: Rect, screen: Rect, size: [u32; 2]) -> Option<(u32, u32, u32, u32)> {
    if screen.width() <= 0.0 || screen.height() <= 0.0 {
        return None;
    }
    let (w, h) = (size[0] as f32, size[1] as f32);
    let sx = w / screen.width();
    let sy = h / screen.height();
    let x0 = ((sel.min.x - screen.min.x) * sx).round().clamp(0.0, w);
    let x1 = ((sel.max.x - screen.min.x) * sx).round().clamp(0.0, w);
    let y0 = ((sel.min.y - screen.min.y) * sy).round().clamp(0.0, h);
    let y1 = ((sel.max.y - screen.min.y) * sy).round().clamp(0.0, h);
    let (pw, ph) = ((x1 - x0) as u32, (y1 - y0) as u32);
    (pw >= MIN_SIDE && ph >= MIN_SIDE).then_some((x0 as u32, y0 as u32, pw, ph))
}

/// Where the result window goes: below the selection if it fits on the monitor, else above, else clamped.
fn result_position(sel: Rect, window: Vec2, monitor: Rect) -> Pos2 {
    let x = sel.min.x.min(monitor.max.x - window.x).max(monitor.min.x);
    let below = sel.max.y + GAP;
    let y = if below + window.y <= monitor.max.y {
        below
    } else {
        (sel.min.y - window.y - GAP).max(monitor.min.y)
    };
    pos2(x, y)
}

/// UV rectangle of `sel` inside the full-screen texture.
fn uv_rect(sel: Rect, screen: Rect) -> Rect {
    let to_uv = |p: Pos2| {
        pos2(
            (p.x - screen.min.x) / screen.width(),
            (p.y - screen.min.y) / screen.height(),
        )
    };
    Rect::from_min_max(to_uv(sel.min), to_uv(sel.max))
}

/// Paints dark pill labels side by side over the screenshot, centred on `x`.
/// `at` is `(x, top if placed below, bottom if placed above)`; it goes above when it would leave `bounds`.
fn pill(
    painter: &Painter,
    p: &Palette,
    bounds: Rect,
    at: (f32, f32, f32),
    parts: &[(&str, FontId)],
) {
    let galleys: Vec<_> = parts
        .iter()
        .map(|(text, font)| {
            painter.layout_no_wrap((*text).to_owned(), font.clone(), p.overlay_text)
        })
        .collect();
    let gaps = space::MD * galleys.len().saturating_sub(1) as f32;
    let content = vec2(
        galleys.iter().map(|g| g.size().x).sum::<f32>() + gaps,
        galleys.iter().map(|g| g.size().y).fold(0.0, f32::max),
    );
    let size = content + vec2(space::MD, space::SM) * 2.0;
    let top = if at.1 + size.y <= bounds.max.y {
        at.1
    } else {
        (at.2 - size.y).max(bounds.min.y)
    };
    let rect = Rect::from_min_size(pos2(at.0 - size.x / 2.0, top), size);
    painter.rect_filled(rect, CornerRadius::same(radius::SURFACE), p.overlay_pill);
    let mut x = rect.min.x + space::MD;
    for g in galleys {
        let w = g.size().x;
        painter.galley(
            pos2(x, rect.center().y - g.size().y / 2.0),
            g,
            p.overlay_text,
        );
        x += w + space::MD;
    }
}

struct Shot {
    image: RgbaImage,
    /// Monitor origin and size in egui points, used to place and size the overlay window.
    pos: Pos2,
    size: Vec2,
    /// Monitor rect in physical pixels (x, y, w, h); only the Windows overlay is placed with it.
    phys: (i32, i32, i32, i32),
}

fn scale_of(m: &xcap::Monitor) -> f32 {
    m.scale_factor()
        .ok()
        .filter(|s| s.is_finite() && *s > 0.0)
        .unwrap_or(1.0)
}

/// Cursor in the OS's own units. On Linux the Xlib call aborts without an X server, so it is skipped there.
fn cursor() -> Option<(i32, i32)> {
    #[cfg(target_os = "linux")]
    {
        let set = |k: &str| std::env::var_os(k).is_some_and(|v| !v.is_empty());
        if set("WAYLAND_DISPLAY") || !set("DISPLAY") {
            return None;
        }
    }
    match Mouse::get_mouse_position() {
        Mouse::Position { x, y } => Some((x, y)),
        Mouse::Error => None,
    }
}

/// xcap's Linux `from_point` multiplies its argument by the scale while the cursor is already physical,
/// so match the point against each monitor's logical rect instead.
#[cfg(target_os = "linux")]
fn monitor_at(x: i32, y: i32) -> Option<xcap::Monitor> {
    xcap::Monitor::all().ok()?.into_iter().find(|m| {
        let s = scale_of(m);
        let (px, py) = (x as f32 / s, y as f32 / s);
        match (m.x(), m.y(), m.width(), m.height()) {
            (Ok(mx), Ok(my), Ok(w), Ok(h)) => {
                let (mx, my) = (mx as f32, my as f32);
                px >= mx && px < mx + w as f32 && py >= my && py < my + h as f32
            }
            _ => false,
        }
    })
}

#[cfg(not(target_os = "linux"))]
fn monitor_at(x: i32, y: i32) -> Option<xcap::Monitor> {
    xcap::Monitor::from_point(x, y).ok()
}

// ponytail: captures only the monitor under the cursor; upgrade: stitch `Monitor::all()`.
fn capture() -> Result<Shot, String> {
    let monitor = match cursor().and_then(|(x, y)| monitor_at(x, y)) {
        Some(m) => m,
        None => {
            let all = xcap::Monitor::all().map_err(|e| e.to_string())?;
            let primary = all
                .iter()
                .position(|m| m.is_primary().unwrap_or(false))
                .unwrap_or(0);
            all.into_iter().nth(primary).ok_or("no monitor found")?
        }
    };
    let mut image = monitor.capture_image().map_err(|e| e.to_string())?;
    imaging::make_opaque(&mut image);
    let scale = scale_of(&monitor);
    let (x, y) = (
        monitor.x().unwrap_or(0) as f32,
        monitor.y().unwrap_or(0) as f32,
    );
    let size = vec2(image.width() as f32 / scale, image.height() as f32 / scale);
    // xcap reports macOS and Linux origins in logical points already, Windows origins in physical pixels.
    let pos = if cfg!(windows) {
        pos2(x / scale, y / scale)
    } else {
        pos2(x, y)
    };
    let phys = (
        monitor.x().unwrap_or(0),
        monitor.y().unwrap_or(0),
        image.width() as i32,
        image.height() as i32,
    );
    Ok(Shot {
        image,
        pos,
        size,
        phys,
    })
}

pub fn run(mode: Mode) -> i32 {
    let lock = match settings::try_lock("snip.lock") {
        Ok(Some(f)) => Some(f),
        Ok(None) => return 0,
        Err(_) => None,
    };
    let cfg = AppSettings::load();
    let mut key = String::new();
    if mode == Mode::Translate {
        match settings::key_get(cfg.provider) {
            Ok(Some(k)) => key = k,
            Ok(None) | Err(_) if cfg.provider == Provider::Custom => {}
            Ok(None) => {
                drop(lock);
                let hint = format!(
                    "Snipslate needs a {} API key to translate.",
                    cfg.provider.display_name()
                );
                match crate::spawn_self(&["--settings", &hint]) {
                    Ok(mut child) => {
                        let _ = child.wait();
                    }
                    Err(e) => {
                        crate::show_message("Snipslate", &format!("Couldn't open settings: {e}"))
                    }
                }
                return 0;
            }
            Err(e) => {
                crate::show_message("Snipslate", &e);
                return 0;
            }
        }
    }

    // Let the tray menu close and the hotkey's keys lift before the screen is frozen.
    std::thread::sleep(Duration::from_millis(150));
    let shot = match capture() {
        Ok(s) => s,
        Err(e) => {
            crate::show_message("Snipslate", &format!("Couldn't capture the screen: {e}"));
            return 0;
        }
    };

    let mut builder = ViewportBuilder::default()
        .with_decorations(false)
        .with_always_on_top()
        .with_taskbar(false)
        .with_visible(false)
        .with_title("Snipslate: select an area")
        .with_position(shot.pos);
    // macOS native fullscreen makes a new Space and animates; on Windows a topmost GL window that exactly
    // covers a monitor is promoted to exclusive fullscreen (display mode change, HDR toggles).
    // Both use a borderless window instead; Windows is placed in physical pixels by `place_overlay`.
    builder = if cfg!(target_os = "linux") {
        builder.with_fullscreen(true)
    } else {
        builder.with_inner_size(shot.size)
    };
    let phys = shot.phys;

    let result = eframe::run_native(
        "Snipslate",
        ui::native_options(builder),
        Box::new(move |cc| {
            ui::theme::install(&cc.egui_ctx);
            Ok(Box::new(SnipApp::new(
                &cc.egui_ctx,
                mode,
                cfg,
                key,
                lock,
                shot.image,
                phys,
            )))
        }),
    );
    match result {
        Ok(()) => 0,
        Err(e) => {
            crate::show_message("Snipslate", &format!("The snip window failed: {e}"));
            1
        }
    }
}

enum Outcome {
    Copied(usize),
    NoText,
    Black,
    Translated(Translation),
    Failed(String),
}

impl Outcome {
    /// Text whose scripts may need fallback fonts, when the result window shows any.
    fn script_text(&self) -> Option<String> {
        match self {
            Outcome::Translated(t) => Some(format!("{}\n{}", t.strict, t.conversational)),
            Outcome::Failed(m) => Some(m.clone()),
            Outcome::Copied(_) | Outcome::NoText | Outcome::Black => None,
        }
    }
}

/// Whether the first letter of `text` is Hebrew or Arabic-script, so the text reads right to left.
fn is_rtl(text: &str) -> bool {
    text.chars().find(|c| c.is_alphabetic()).is_some_and(|c| {
        matches!(
            c as u32,
            0x0590..=0x05FF
                | 0x0600..=0x06FF
                | 0x0750..=0x077F
                | 0x08A0..=0x08FF
                | 0xFB50..=0xFDFF
                | 0xFE70..=0xFEFF
        )
    })
}

/// Stable id of a translation text field, so Ctrl+C can tell whether one has focus.
fn text_id(strict: bool) -> Id {
    Id::unique(if strict {
        "snipslate-strict"
    } else {
        "snipslate-translation"
    })
}

enum Phase {
    Select,
    Blank,
    Busy(&'static str),
    Done(Outcome),
}

/// Outcome of the last Copy click, shown beside the footer buttons.
enum Status {
    Ok(String),
    Err(String),
}

/// What a result-window click or shortcut asked for.
enum Act {
    Copy,
    CopyStrict,
    CopyClose,
    Retry,
    Settings,
    Close,
}

struct FullShot {
    image: RgbaImage,
    tex: TextureHandle,
}

struct SnipApp {
    mode: Mode,
    cfg: AppSettings,
    key: String,
    /// Held during selection only, so the next snip can start while this result is open.
    lock: Option<File>,
    shot: Option<FullShot>,
    crop: Option<RgbaImage>,
    phase: Phase,
    shown: bool,
    drag_start: Option<Pos2>,
    sel: Option<Rect>,
    rx: Option<Receiver<Outcome>>,
    close_at: Option<Instant>,
    status: Option<Status>,
    thumb: Option<TextureHandle>,
    header: String,
    focus_set: bool,
    /// Result-window size, position and min-size flag, held until fullscreen has really ended.
    pending: Option<(Vec2, Pos2, bool)>,
    /// Selection and monitor rects (outer-window coordinates), to place a grown window later.
    anchor: Option<(Rect, Rect)>,
    pending_frames: u32,
    phys: (i32, i32, i32, i32),
}

/// Puts the hidden overlay exactly over its monitor in physical pixels, one row taller.
/// Logical placement through winit misplaces it on mixed-DPI setups.
#[cfg(windows)]
fn place_overlay(frame: &eframe::Frame, (x, y, w, h): (i32, i32, i32, i32)) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SetWindowPos,
    };
    use winit::raw_window_handle::{HasWindowHandle as _, RawWindowHandle};

    let Ok(handle) = frame.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(win) = handle.as_raw() else {
        return;
    };
    let hwnd = HWND(win.hwnd.get() as *mut _);
    // ponytail: 1 px overhang beats the fullscreen-promotion heuristic; it may show on a monitor stacked directly below.
    // Twice: a move onto a monitor with another DPI scale sends WM_DPICHANGED during the first call, and
    // winit answers it by resizing to Windows' suggested rect. The second call lands on the same monitor.
    for _ in 0..2 {
        // SAFETY: `hwnd` is the live window of `frame`, and SetWindowPos only reads its integer arguments.
        let placed = unsafe {
            SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                x,
                y,
                w,
                h + 1,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )
        };
        if let Err(e) = placed {
            eprintln!("snipslate: SetWindowPos failed: {e}");
            return;
        }
    }
}
#[cfg(not(windows))]
fn place_overlay(_frame: &eframe::Frame, _phys: (i32, i32, i32, i32)) {}

/// Frames to wait for the window manager to leave fullscreen before resizing anyway.
const FULLSCREEN_WAIT_FRAMES: u32 = 10;

impl SnipApp {
    fn new(
        ctx: &egui::Context,
        mode: Mode,
        cfg: AppSettings,
        key: String,
        lock: Option<File>,
        image: RgbaImage,
        phys: (i32, i32, i32, i32),
    ) -> Self {
        // Uploaded once; ponytail: a monitor wider than the GPU's max texture size (commonly 16384 px) fails here.
        let size = [image.width() as usize, image.height() as usize];
        let tex = ctx.load_texture(
            "shot",
            ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
            TextureOptions::LINEAR,
        );
        let header = format!("{} \u{b7} {}", cfg.provider.display_name(), cfg.model);
        Self {
            mode,
            cfg,
            key,
            lock,
            shot: Some(FullShot { image, tex }),
            crop: None,
            phase: Phase::Select,
            shown: false,
            drag_start: None,
            sel: None,
            rx: None,
            close_at: None,
            status: None,
            thumb: None,
            header,
            focus_set: false,
            pending: None,
            anchor: None,
            pending_frames: 0,
            phys,
        }
    }

    fn spawn(
        &mut self,
        ctx: &egui::Context,
        label: &'static str,
        work: impl FnOnce() -> Outcome + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.phase = Phase::Busy(label);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(work());
            ctx.request_repaint();
        });
    }

    fn start_copy(&mut self, ctx: &egui::Context, crop: RgbaImage) {
        self.spawn(ctx, "Reading text\u{2026}", move || {
            match ocr::read_text(&crop) {
                Err(e) => Outcome::Failed(e),
                Ok(t) if t.is_empty() => {
                    if imaging::is_mostly_black(&crop) {
                        Outcome::Black
                    } else {
                        Outcome::NoText
                    }
                }
                Ok(t) => match clip::set_text(&t) {
                    Ok(()) => Outcome::Copied(t.chars().count()),
                    Err(e) => Outcome::Failed(e),
                },
            }
        });
    }

    fn start_translate(&mut self, ctx: &egui::Context, crop: RgbaImage) {
        let cfg = self.cfg.clone();
        let api_key = self.key.clone();
        self.spawn(ctx, "Translating\u{2026}", move || {
            let input = if cfg.text_only {
                match ocr::read_text(&crop) {
                    Err(e) => return Outcome::Failed(e),
                    Ok(t) if t.is_empty() => return Outcome::NoText,
                    Ok(t) => Input::Text(t),
                }
            } else {
                match imaging::to_png(&crop) {
                    Ok(p) => Input::Png(p),
                    Err(e) => return Outcome::Failed(e),
                }
            };
            let job = TranslateJob {
                provider: cfg.provider,
                base_url: cfg.resolve_base_url(),
                model: cfg.model.clone(),
                api_key,
                target_language: cfg.target_language.clone(),
                input,
            };
            match translator::translate(&job) {
                Ok(t) => Outcome::Translated(t),
                Err(e) => Outcome::Failed(e),
            }
        });
    }

    fn poll(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.rx else { return };
        let Ok(outcome) = rx.try_recv() else { return };
        self.rx = None;
        if let Some(text) = outcome.script_text() {
            ui::install_fallback_fonts(ctx, &text);
            if self.mode == Mode::Copy {
                self.grow_to_result(ctx);
            }
        } else {
            self.close_at = Some(Instant::now() + AUTO_CLOSE);
        }
        self.phase = Phase::Done(outcome);
    }

    /// Crops the selection and turns the overlay window into the result window.
    fn finish_selection(&mut self, ctx: &egui::Context, sel: Rect, screen: Rect) {
        let Some(full) = self.shot.take() else { return };
        self.lock = None;
        let Some((x, y, w, h)) =
            selection_to_pixels(sel, screen, [full.image.width(), full.image.height()])
        else {
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        };
        let crop = image::imageops::crop_imm(&full.image, x, y, w, h).to_image();
        drop(full);
        if self.mode == Mode::Translate {
            let dims = [crop.width() as usize, crop.height() as usize];
            self.thumb = Some(ctx.load_texture(
                "thumb",
                ColorImage::from_rgba_unmultiplied(dims, crop.as_raw()),
                TextureOptions::LINEAR,
            ));
        }

        let outer = ctx.input(|i| i.viewport().outer_rect).unwrap_or(screen);
        let offset = outer.min - screen.min;
        let size = match self.mode {
            Mode::Copy => TOAST_SIZE,
            Mode::Translate => RESULT_SIZE,
        };
        let pos = result_position(sel.translate(offset), size, outer);
        self.anchor = Some((sel.translate(offset), outer));
        // Leaving fullscreen is asynchronous, so the geometry waits (see `apply_pending`).
        // Only Linux enters fullscreen; macOS and Windows use a plain borderless window.
        let min = self.mode == Mode::Translate;
        ctx.send_viewport_cmd(ViewportCommand::Decorations(true));
        if !cfg!(target_os = "linux") {
            Self::send_geometry(ctx, size, pos, min);
        } else {
            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
            self.pending = Some((size, pos, min));
            self.pending_frames = 0;
            ctx.request_repaint();
        }

        match self.mode {
            Mode::Copy => {
                self.crop = Some(crop.clone());
                self.start_copy(ctx, crop);
            }
            Mode::Translate if imaging::is_mostly_black(&crop) => {
                self.crop = Some(crop);
                self.phase = Phase::Blank;
            }
            Mode::Translate => {
                self.crop = Some(crop.clone());
                self.start_translate(ctx, crop);
            }
        }
    }

    /// The copy toast is too small for an error: grow it to the result size, re-placed so it stays
    /// on the monitor. On Linux the toast geometry may still be pending; replace it instead of
    /// letting `apply_pending` shrink the window back afterwards.
    fn grow_to_result(&mut self, ctx: &egui::Context) {
        let Some((sel, monitor)) = self.anchor else {
            return;
        };
        let pos = result_position(sel, RESULT_SIZE, monitor);
        if self.pending.is_some() {
            self.pending = Some((RESULT_SIZE, pos, true));
        } else {
            Self::send_geometry(ctx, RESULT_SIZE, pos, true);
        }
    }

    fn send_geometry(ctx: &egui::Context, size: Vec2, pos: Pos2, min: bool) {
        ctx.send_viewport_cmd(ViewportCommand::Resizable(true));
        ctx.send_viewport_cmd(ViewportCommand::Title("Snipslate".to_owned()));
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
        if min {
            ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(RESULT_MIN));
        }
        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos));
        ctx.send_viewport_cmd(ViewportCommand::WindowLevel(WindowLevel::AlwaysOnTop));
    }

    /// Sends the result-window geometry once fullscreen is off, or after a bounded wait.
    fn apply_pending(&mut self, ctx: &egui::Context) {
        let Some((size, pos, min)) = self.pending else {
            return;
        };
        self.pending_frames += 1;
        let left = ctx.input(|i| i.viewport().fullscreen) == Some(false);
        if left || self.pending_frames >= FULLSCREEN_WAIT_FRAMES {
            self.pending = None;
            Self::send_geometry(ctx, size, pos, min);
        }
        ctx.request_repaint();
    }

    fn select_ui(&mut self, ctx: &egui::Context, painter: &egui::Painter) {
        // On Windows the window is one physical row taller than the monitor, so map only the monitor's part.
        let content = ctx.content_rect();
        let screen = match &self.shot {
            Some(full) if cfg!(windows) => {
                let px = vec2(full.image.width() as f32, full.image.height() as f32);
                Rect::from_min_size(content.min, px / ctx.pixels_per_point())
            }
            _ => content,
        };
        let (esc, right, pressed, latest, down, released) = ctx.input(|i| {
            (
                i.key_pressed(Key::Escape),
                i.pointer.button_pressed(PointerButton::Secondary),
                i.pointer
                    .primary_pressed()
                    .then(|| i.pointer.press_origin())
                    .flatten(),
                i.pointer.latest_pos(),
                i.pointer.primary_down(),
                i.pointer.primary_released(),
            )
        });
        if esc || right {
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }
        if let Some(p) = pressed {
            self.drag_start = Some(p);
            self.sel = None;
        }
        if let (Some(start), Some(now)) = (self.drag_start, latest)
            && (down || released)
        {
            self.sel = Some(Rect::from_two_pos(start, now).intersect(screen));
        }
        ctx.set_cursor_icon(CursorIcon::Crosshair);

        let Some(full) = &self.shot else { return };
        let id = full.tex.id();
        let image_px = [full.image.width(), full.image.height()];
        let p = theme::palette(ctx);
        let uv_all = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        painter.image(id, screen, uv_all, Color32::WHITE);
        painter.rect_filled(screen, 0.0, p.scrim);
        if let Some(sel) = self.sel {
            painter.image(id, sel, uv_rect(sel, screen), Color32::WHITE);
            painter.rect_stroke(
                sel,
                0.0,
                Stroke::new(OUTLINE, p.accent),
                StrokeKind::Outside,
            );
            for corner in [
                sel.left_top(),
                sel.right_top(),
                sel.left_bottom(),
                sel.right_bottom(),
            ] {
                let handle = Rect::from_center_size(corner, Vec2::splat(HANDLE));
                painter.rect_filled(handle, 0.0, p.surface);
                painter.rect_stroke(
                    handle,
                    0.0,
                    Stroke::new(OUTLINE, p.accent),
                    StrokeKind::Middle,
                );
            }
            if let Some((_, _, w, h)) = selection_to_pixels(sel, screen, image_px) {
                let text = format!("{w} \u{d7} {h}");
                let font = FontId::proportional(size::CAPTION);
                pill(
                    painter,
                    &p,
                    screen,
                    (sel.center().x, sel.max.y + GAP, sel.min.y - GAP),
                    &[(text.as_str(), font)],
                );
            }
        } else if self.drag_start.is_none() {
            let mode = match self.mode {
                Mode::Translate => "Translate",
                Mode::Copy => "Copy text",
            };
            let body = FontId::proportional(size::BODY);
            pill(
                painter,
                &p,
                screen,
                (screen.center().x, screen.min.y + HINT_TOP, 0.0),
                &[(mode, theme::strong(size::BODY)), (HINT, body)],
            );
        }

        if released {
            match self.sel {
                Some(sel) => self.finish_selection(ctx, sel, screen),
                None => ctx.send_viewport_cmd(ViewportCommand::Close),
            }
        }
    }

    fn blank_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let p = theme::palette(ctx);
        Frame::new()
            .inner_margin(Margin::same(space::LG as i8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    icons::show(ui, Icon::Alert, p.text);
                    ui.label(RichText::new("This area looks blank").font(theme::strong(size::BODY)));
                });
                ui.label(theme::muted(
                    ctx,
                    format!(
                        "It's almost entirely black. {DRM} always captures as black, so there is nothing to translate. \
                         Tip: use the player's own subtitle settings."
                    ),
                ));
                ui.label(theme::muted(
                    ctx,
                    "If the area really has text on a black background, you can send it anyway.",
                ));
                ui.horizontal(|ui| {
                    if ui.button("Send anyway").clicked()
                        && let Some(crop) = self.crop.clone()
                    {
                        self.start_translate(ctx, crop);
                    }
                    let cancel = ui.button("Cancel");
                    if !self.focus_set {
                        self.focus_set = true;
                        cancel.request_focus();
                    }
                    if cancel.clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                });
            });
    }

    /// Copy-mode result: one icon, a title and a caption.
    fn toast_ui(&self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let p = theme::palette(ctx);
        let (icon, color, title, note) = match &self.phase {
            Phase::Done(Outcome::Copied(n)) => {
                let unit = if *n == 1 { "character" } else { "characters" };
                (
                    Some(Icon::CheckCircle),
                    p.success,
                    format!("Copied {n} {unit}"),
                    "Paste it anywhere.".to_owned(),
                )
            }
            Phase::Done(Outcome::NoText) => (
                Some(Icon::Alert),
                p.muted,
                "No text found".to_owned(),
                "Try a larger area or a clearer font.".to_owned(),
            ),
            Phase::Done(Outcome::Black) => (
                Some(Icon::Alert),
                p.muted,
                "That area is black".to_owned(),
                format!("{DRM} can't be captured."),
            ),
            Phase::Busy(label) => (None, p.muted, (*label).to_owned(), String::new()),
            _ => return,
        };
        Frame::new()
            .inner_margin(Margin::symmetric(space::LG as i8, space::MD as i8))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    match icon {
                        Some(icon) => {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(space::XL), Sense::hover());
                            icons::paint(ui.painter(), rect, icon, color);
                        }
                        None => {
                            ui.add(egui::Spinner::new().size(space::XL));
                        }
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = space::XS;
                        ui.label(RichText::new(title).font(theme::strong(size::BODY)));
                        if !note.is_empty() {
                            ui.label(theme::caption(ctx, note));
                        }
                    });
                });
            });
    }

    /// Thumbnail plus title and provider line (Translate mode, busy or done).
    fn header_ui(&self, ui: &mut egui::Ui, p: &Palette) {
        ui.horizontal(|ui| {
            if let Some(tex) = &self.thumb {
                let src = tex.size_vec2();
                let fit = (THUMB.x / src.x).min(THUMB.y / src.y);
                let corner = CornerRadius::same(radius::CONTROL);
                let shown =
                    ui.add(egui::Image::from_texture((tex.id(), src * fit)).corner_radius(corner));
                ui.painter().rect_stroke(
                    shown.rect,
                    corner,
                    Stroke::new(HAIRLINE, p.border),
                    StrokeKind::Inside,
                );
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = space::XS;
                match &self.phase {
                    Phase::Busy(label) => {
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new().size(size::BODY));
                            ui.label(RichText::new(*label).font(theme::strong(size::BODY)));
                        });
                    }
                    _ => {
                        ui.label(
                            RichText::new(format!("Translated to {}", self.cfg.target_language))
                                .font(theme::strong(size::BODY)),
                        );
                    }
                }
                ui.label(theme::caption(
                    ui.ctx(),
                    format!("{} \u{b7} AI translation", self.header),
                ));
            });
        });
    }

    /// One translation: title (and note) with an icon-only copy button, then the selectable text.
    /// Returns true when the copy button was clicked.
    fn section_ui(
        ui: &mut egui::Ui,
        title: &str,
        note: Option<&str>,
        text: &str,
        strict: bool,
        copy_name: &str,
    ) -> bool {
        let mut copy = false;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = space::XS;
                ui.label(RichText::new(title).font(theme::strong(size::BODY)));
                if let Some(note) = note {
                    ui.label(theme::caption(ui.ctx(), note));
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                copy = icons::button(ui, Icon::Copy, "", copy_name).clicked();
            });
        });
        ui.add_space(space::SM);
        let mut shown: &str = text;
        ui.add(
            TextEdit::multiline(&mut shown)
                .id(text_id(strict))
                .font(theme::reading())
                .horizontal_align(if is_rtl(text) {
                    Align::RIGHT
                } else {
                    Align::LEFT
                })
                .desired_width(f32::INFINITY)
                .frame(Frame::NONE),
        );
        copy
    }

    /// The translation to show and copy, unless the model found no text.
    fn translation(&self) -> Option<&Translation> {
        match &self.phase {
            Phase::Done(Outcome::Translated(t)) if !t.is_no_text() => Some(t),
            _ => None,
        }
    }

    fn body_ui(&self, ui: &mut egui::Ui, p: &Palette) -> Option<Act> {
        let mut act = None;
        match &self.phase {
            Phase::Busy(_) => {
                self.header_ui(ui, p);
                ui.add_space(space::SM);
                let line = ui.text_style_height(&theme::reading());
                for group in 0..2 {
                    if group > 0 {
                        ui.add_space(space::MD);
                    }
                    for share in SKELETON {
                        let (bar, _) = ui.allocate_exact_size(
                            vec2(ui.available_width() * share, line),
                            Sense::hover(),
                        );
                        ui.painter()
                            .rect_filled(bar, CornerRadius::same(radius::CONTROL), p.hover);
                    }
                }
            }
            Phase::Done(Outcome::Translated(t)) if t.is_no_text() => {
                ui.horizontal(|ui| {
                    icons::show(ui, Icon::Alert, p.muted);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = space::XS;
                        ui.label(RichText::new("No text found").font(theme::strong(size::BODY)));
                        ui.label(theme::caption(
                            ui.ctx(),
                            "Try a larger area or a clearer font.",
                        ));
                    });
                });
            }
            Phase::Done(Outcome::Translated(t)) => {
                self.header_ui(ui, p);
                ui.add_space(space::SM);
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if t.is_single() {
                            if Self::section_ui(
                                ui,
                                "Translation",
                                None,
                                &t.conversational,
                                false,
                                "Copy translation",
                            ) {
                                act = Some(Act::Copy);
                            }
                            return;
                        }
                        if Self::section_ui(
                            ui,
                            "Conversational",
                            Some("How a native speaker would say it"),
                            &t.conversational,
                            false,
                            "Copy conversational translation",
                        ) {
                            act = Some(Act::Copy);
                        }
                        ui.add_space(space::MD);
                        let (rule, _) = ui.allocate_exact_size(
                            vec2(ui.available_width(), HAIRLINE),
                            Sense::hover(),
                        );
                        ui.painter().hline(
                            rule.x_range(),
                            rule.center().y,
                            Stroke::new(HAIRLINE, p.border),
                        );
                        ui.add_space(space::MD);
                        if Self::section_ui(
                            ui,
                            "Strict",
                            Some("Close to the original wording"),
                            &t.strict,
                            true,
                            "Copy strict translation",
                        ) {
                            act = Some(Act::CopyStrict);
                        }
                    });
            }
            Phase::Done(Outcome::Failed(msg)) => {
                let title = match self.mode {
                    Mode::Translate => "Translation failed",
                    Mode::Copy => "Couldn't read text",
                };
                Frame::new()
                    .fill(p.danger_bg)
                    .stroke(Stroke::new(HAIRLINE, p.danger))
                    .corner_radius(CornerRadius::same(radius::CARD))
                    .inner_margin(Margin::same(space::MD as i8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            icons::show(ui, Icon::Alert, p.danger);
                            ui.label(
                                RichText::new(title)
                                    .font(theme::strong(size::BODY))
                                    .color(p.danger),
                            );
                        });
                        ScrollArea::vertical()
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                let mut shown: &str = msg;
                                ui.add(
                                    TextEdit::multiline(&mut shown)
                                        .desired_width(f32::INFINITY)
                                        .frame(Frame::NONE),
                                );
                            });
                    });
            }
            _ => {}
        }
        act
    }

    fn footer_ui(&self, ui: &mut egui::Ui, p: &Palette) -> Option<Act> {
        let mut act = None;
        match &self.phase {
            Phase::Done(Outcome::Translated(_)) => {
                if self.translation().is_some() {
                    let tip = if cfg!(target_os = "macos") {
                        "Copy (Cmd+C)"
                    } else {
                        "Copy (Ctrl+C)"
                    };
                    let copy = theme::primary_scope(ui, |ui| {
                        icons::button(ui, Icon::Copy, "Copy", "Copy")
                    });
                    if copy.on_hover_text(tip).clicked() {
                        act = Some(Act::Copy);
                    }
                }
            }
            Phase::Done(Outcome::Failed(_)) => {
                if self.crop.is_some()
                    && theme::primary_scope(ui, |ui| {
                        icons::button(ui, Icon::Refresh, "Retry", "Retry")
                    })
                    .clicked()
                {
                    act = Some(Act::Retry);
                }
                if icons::button(ui, Icon::Settings, "Open settings", "Open settings").clicked() {
                    act = Some(Act::Settings);
                }
            }
            _ => {}
        }
        let close = icons::button(ui, Icon::Close, "Close", "Close");
        if close.on_hover_text("Close (Esc)").clicked() {
            act = Some(Act::Close);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if let Some(status) = &self.status {
                let (icon, color, text) = match status {
                    Status::Ok(t) => (Icon::Check, p.success, t),
                    Status::Err(t) => (Icon::Alert, p.danger, t),
                };
                ui.label(RichText::new(text).color(color));
                icons::show(ui, icon, color);
            }
        });
        act
    }

    /// Translate-mode window and every Copy-mode failure: body, hairline, footer.
    fn result_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let p = theme::palette(ctx);
        let full = ui.max_rect();
        let split = full.max.y - (ui.spacing().interact_size.y + 2.0 * space::MD);
        let body = Rect::from_min_max(full.min, pos2(full.max.x, split));
        let footer = Rect::from_min_max(pos2(full.min.x, split), full.max)
            .shrink2(vec2(space::LG, space::MD));
        let mut act = None;
        ui.scope_builder(UiBuilder::new().max_rect(body), |ui| {
            act = Frame::new()
                .inner_margin(Margin::same(space::LG as i8))
                .show(ui, |ui| self.body_ui(ui, &p))
                .inner;
        });
        ui.painter()
            .hline(full.x_range(), split, Stroke::new(HAIRLINE, p.border));
        let footer_builder = UiBuilder::new()
            .max_rect(footer)
            .layout(Layout::left_to_right(Align::Center));
        ui.scope_builder(footer_builder, |ui| {
            if let Some(clicked) = self.footer_ui(ui, &p) {
                act = Some(clicked);
            }
        });

        if self.translation().is_some() {
            let (enter, copy) = ctx.input(|i| {
                (
                    i.key_pressed(Key::Enter),
                    i.events.iter().any(|e| matches!(e, egui::Event::Copy)),
                )
            });
            let any_focus = ctx.memory(|m| m.focused().is_some());
            let text_focus =
                ctx.memory(|m| m.has_focus(text_id(false)) || m.has_focus(text_id(true)));
            if enter && !any_focus {
                act = Some(Act::CopyClose);
            } else if copy && !text_focus {
                act = Some(Act::Copy);
            }
        }
        if let Some(act) = act {
            self.apply(ctx, act);
        }
    }

    fn apply(&mut self, ctx: &egui::Context, act: Act) {
        match act {
            Act::Copy | Act::CopyStrict | Act::CopyClose => {
                let Some(t) = self.translation() else {
                    return;
                };
                let (text, done) = if matches!(act, Act::CopyStrict) {
                    (&t.strict, "Copied strict translation")
                } else {
                    (&t.conversational, "Copied")
                };
                match clip::set_text(text) {
                    Ok(()) if matches!(act, Act::CopyClose) => {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                    Ok(()) => self.status = Some(Status::Ok(done.to_owned())),
                    Err(e) => self.status = Some(Status::Err(e)),
                }
            }
            Act::Retry => {
                if let Some(crop) = self.crop.clone() {
                    self.status = None;
                    match self.mode {
                        Mode::Translate => self.start_translate(ctx, crop),
                        Mode::Copy => self.start_copy(ctx, crop),
                    }
                }
            }
            Act::Settings => match crate::spawn_self(&["--settings"]) {
                // Stay alive, hidden, until Settings exits: the tray reloads hotkeys when this
                // child ends, so closing now would skip applying a shortcut changed there.
                Ok(mut child) => {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(false));
                    let ctx = ctx.clone();
                    std::thread::spawn(move || {
                        if let Err(e) = child.wait() {
                            eprintln!("snipslate: waiting for settings failed: {e}");
                        }
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                        ctx.request_repaint();
                    });
                }
                Err(e) => self.status = Some(Status::Err(e.to_string())),
            },
            Act::Close => ctx.send_viewport_cmd(ViewportCommand::Close),
        }
    }
}

impl eframe::App for SnipApp {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Runs even while the window is hidden, so the first pass can reveal it after the texture is ready.
        if !self.shown {
            self.shown = true;
            if cfg!(windows) {
                place_overlay(frame, self.phys);
            }
            ctx.send_viewport_cmd(ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(ViewportCommand::Focus);
            ctx.request_repaint();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll(&ctx);
        self.apply_pending(&ctx);
        if matches!(self.phase, Phase::Select) {
            let painter = ui.painter().clone();
            self.select_ui(&ctx, &painter);
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        if let Some(at) = self.close_at {
            let now = Instant::now();
            if now >= at {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            } else {
                ctx.request_repaint_after(at - now);
            }
        }
        let p = theme::palette(&ctx);
        let blank = matches!(self.phase, Phase::Blank);
        let result = matches!(
            (&self.phase, self.mode),
            (Phase::Busy(_), Mode::Translate)
                | (Phase::Done(Outcome::Translated(_) | Outcome::Failed(_)), _)
        );
        egui::CentralPanel::default()
            .frame(Frame::new().fill(p.bg))
            .show(ui, |ui| {
                if blank {
                    self.blank_ui(ui, &ctx);
                } else if result {
                    self.result_ui(ui, &ctx);
                } else {
                    self.toast_ui(ui, &ctx);
                }
            });
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        if matches!(self.phase, Phase::Select) {
            Color32::BLACK.to_normalized_gamma_f32()
        } else {
            visuals.panel_fill.to_normalized_gamma_f32()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(pos2(x0, y0), pos2(x1, y1))
    }

    #[test]
    fn pixels_identity_at_scale_one() {
        let screen = rect(0.0, 0.0, 1000.0, 500.0);
        assert_eq!(
            selection_to_pixels(rect(10.0, 20.0, 110.0, 70.0), screen, [1000, 500]),
            Some((10, 20, 100, 50))
        );
    }

    #[test]
    fn pixels_scale_with_dpi() {
        let screen = rect(0.0, 0.0, 1000.0, 500.0);
        assert_eq!(
            selection_to_pixels(rect(10.0, 20.0, 110.0, 70.0), screen, [2000, 1000]),
            Some((20, 40, 200, 100))
        );
    }

    #[test]
    fn pixels_clamped_to_image() {
        let screen = rect(0.0, 0.0, 100.0, 100.0);
        assert_eq!(
            selection_to_pixels(rect(-20.0, 50.0, 300.0, 400.0), screen, [100, 100]),
            Some((0, 50, 100, 50))
        );
    }

    #[test]
    fn pixels_too_small_is_none() {
        let screen = rect(0.0, 0.0, 100.0, 100.0);
        assert_eq!(
            selection_to_pixels(rect(10.0, 10.0, 13.0, 90.0), screen, [100, 100]),
            None
        );
        assert_eq!(
            selection_to_pixels(rect(10.0, 10.0, 90.0, 13.0), screen, [100, 100]),
            None
        );
    }

    #[test]
    fn pixels_off_origin_screen() {
        let screen = rect(100.0, 50.0, 200.0, 150.0);
        assert_eq!(
            selection_to_pixels(rect(110.0, 60.0, 150.0, 100.0), screen, [200, 200]),
            Some((20, 20, 80, 80))
        );
    }

    #[test]
    fn uv_maps_into_unit_square() {
        let uv = uv_rect(
            rect(100.0, 50.0, 150.0, 100.0),
            rect(100.0, 50.0, 200.0, 150.0),
        );
        assert_eq!(uv, rect(0.0, 0.0, 0.5, 0.5));
    }

    #[test]
    fn position_below_when_it_fits() {
        let monitor = rect(0.0, 0.0, 1920.0, 1080.0);
        let p = result_position(
            rect(100.0, 100.0, 300.0, 200.0),
            vec2(440.0, 220.0),
            monitor,
        );
        assert_eq!(p, pos2(100.0, 208.0));
    }

    #[test]
    fn position_above_when_below_does_not_fit() {
        let monitor = rect(0.0, 0.0, 1920.0, 1080.0);
        let p = result_position(
            rect(100.0, 900.0, 300.0, 1000.0),
            vec2(440.0, 220.0),
            monitor,
        );
        assert_eq!(p, pos2(100.0, 672.0));
    }

    #[test]
    fn position_clamped_at_right_edge_and_top() {
        let monitor = rect(0.0, 0.0, 1920.0, 1080.0);
        let p = result_position(
            rect(1800.0, 10.0, 1900.0, 1070.0),
            vec2(440.0, 220.0),
            monitor,
        );
        assert_eq!(p, pos2(1480.0, 0.0));
    }

    #[test]
    fn rtl_follows_the_first_letter() {
        assert!(is_rtl("שלום עולם"));
        assert!(is_rtl("مرحبا بالعالم"));
        assert!(is_rtl("42. مرحبا"));
        assert!(is_rtl("\u{FB50}\u{FE70}"));
        assert!(!is_rtl("Hello مرحبا"));
        assert!(!is_rtl("Bonjour"));
        assert!(!is_rtl("12 34"));
        assert!(!is_rtl(""));
    }
}
