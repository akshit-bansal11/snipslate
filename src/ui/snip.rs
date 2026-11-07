//! Snip child process: freezes one monitor, lets the user drag a box, then shows the OCR / translation
//! result in the same window. One `run_native`; the screenshot is uploaded as a texture exactly once.

use std::fs::File;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Color32, ColorImage, CursorIcon, FontId, Key, PointerButton, Pos2, Rect, ScrollArea,
    Stroke, StrokeKind, TextEdit, TextureHandle, TextureOptions, Vec2, ViewportBuilder,
    ViewportCommand, WindowLevel, pos2, vec2,
};
use image::RgbaImage;
use mouse_position::mouse_position::Mouse;

use crate::settings::{self, AppSettings};
use crate::translator::{self, Input, Provider, TranslateJob};
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
const ACCENT: Color32 = Color32::from_rgb(0, 120, 215);
const ERROR: Color32 = Color32::from_rgb(200, 40, 40);

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
    Translated(String),
    Failed(String),
}

enum Phase {
    Select,
    Blank,
    Busy(&'static str),
    Done(Outcome),
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
    status: String,
    header: String,
    focus_set: bool,
    /// Result-window size, position and min-size flag, held until fullscreen has really ended.
    pending: Option<(Vec2, Pos2, bool)>,
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
            status: String::new(),
            header,
            focus_set: false,
            pending: None,
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
        match &outcome {
            Outcome::Translated(t) | Outcome::Failed(t) => ui::install_fallback_fonts(ctx, t),
            Outcome::Copied(_) | Outcome::NoText | Outcome::Black => {
                self.close_at = Some(Instant::now() + AUTO_CLOSE)
            }
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

        let outer = ctx.input(|i| i.viewport().outer_rect).unwrap_or(screen);
        let offset = outer.min - screen.min;
        let size = match self.mode {
            Mode::Copy => vec2(300.0, 90.0),
            Mode::Translate => vec2(440.0, 220.0),
        };
        let pos = result_position(sel.translate(offset), size, outer);
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
            Mode::Copy => self.start_copy(ctx, crop),
            Mode::Translate if imaging::is_mostly_black(&crop) => {
                self.crop = Some(crop);
                self.phase = Phase::Blank;
            }
            Mode::Translate => self.start_translate(ctx, crop),
        }
    }

    fn send_geometry(ctx: &egui::Context, size: Vec2, pos: Pos2, min: bool) {
        ctx.send_viewport_cmd(ViewportCommand::Resizable(true));
        ctx.send_viewport_cmd(ViewportCommand::Title("Snipslate".to_owned()));
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
        if min {
            ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(vec2(260.0, 150.0)));
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
        let uv_all = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        painter.image(id, screen, uv_all, Color32::WHITE);
        painter.rect_filled(screen, 0.0, Color32::from_black_alpha(110));
        if let Some(sel) = self.sel {
            painter.image(id, sel, uv_rect(sel, screen), Color32::WHITE);
            painter.rect_stroke(sel, 0.0, Stroke::new(2.0, ACCENT), StrokeKind::Outside);
        } else if self.drag_start.is_none() {
            let galley =
                painter.layout_no_wrap(HINT.to_owned(), FontId::proportional(16.0), Color32::WHITE);
            let at = pos2(screen.center().x, screen.min.y + 44.0);
            let pill = Rect::from_center_size(at, galley.size()).expand2(vec2(14.0, 7.0));
            painter.rect_filled(pill, 8.0, Color32::from_black_alpha(190));
            painter.galley(pill.center() - galley.size() / 2.0, galley, Color32::WHITE);
        }

        if released {
            match self.sel {
                Some(sel) => self.finish_selection(ctx, sel, screen),
                None => ctx.send_viewport_cmd(ViewportCommand::Close),
            }
        }
    }

    fn blank_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("This area looks blank");
        ui.add_space(6.0);
        ui.label(format!(
            "It's almost entirely black. {DRM} always captures as black, so there is nothing to translate. \
             Tip: use the player's own subtitle settings."
        ));
        ui.add_space(6.0);
        ui.label("If the area really has text on a black background, you can send it anyway.");
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button("Send anyway").clicked()
                && let Some(crop) = self.crop.take()
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
    }

    fn done_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let translate = self.mode == Mode::Translate;
        let mut copy_text: Option<String> = None;
        let mut close = false;
        let Phase::Done(outcome) = &self.phase else {
            return;
        };

        if translate {
            ui.weak(&self.header);
        }
        let body_h = (ui.available_height() - 40.0).max(30.0);
        match outcome {
            Outcome::Copied(n) => {
                ui.label(format!("Text copied ({n} characters)"));
            }
            Outcome::NoText => {
                ui.label("No text found");
            }
            Outcome::Black => {
                ui.label(format!(
                    "Nothing to read: that area is black. {DRM} can't be captured."
                ));
            }
            Outcome::Translated(text) => {
                ScrollArea::vertical()
                    .max_height(body_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut shown: &str = text;
                        ui.add(
                            TextEdit::multiline(&mut shown)
                                .desired_width(f32::INFINITY)
                                .frame(egui::Frame::NONE),
                        );
                    });
            }
            Outcome::Failed(msg) => {
                ScrollArea::vertical()
                    .max_height(body_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.colored_label(ERROR, msg);
                    });
            }
        }
        let auto = matches!(
            outcome,
            Outcome::Copied(_) | Outcome::NoText | Outcome::Black
        );
        if !auto {
            ui.separator();
            ui.horizontal(|ui| {
                if translate {
                    let text = if let Outcome::Translated(t) = outcome {
                        Some(t)
                    } else {
                        None
                    };
                    if ui
                        .add_enabled(text.is_some(), egui::Button::new("Copy"))
                        .clicked()
                    {
                        copy_text = text.cloned();
                    }
                }
                if ui.button("Close").clicked() {
                    close = true;
                }
                if translate && matches!(outcome, Outcome::Failed(_)) {
                    ui.colored_label(ERROR, "Translation failed");
                } else {
                    ui.weak(&self.status);
                }
            });
        }
        if let Some(text) = copy_text {
            self.status = match clip::set_text(&text) {
                Ok(()) => "Copied to the clipboard".to_owned(),
                Err(e) => e,
            };
        }
        if close {
            ctx.send_viewport_cmd(ViewportCommand::Close);
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
        egui::CentralPanel::default().show(ui, |ui| match &self.phase {
            Phase::Blank => self.blank_ui(ui, &ctx),
            Phase::Busy(label) => {
                ui.label(*label);
            }
            Phase::Done(_) => self.done_ui(ui, &ctx),
            Phase::Select => {}
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
}
