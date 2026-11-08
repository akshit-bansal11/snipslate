//! The `--message` window: wrapped text and one OK button.

use eframe::egui::{self, Align, Frame, Layout, Margin};

use super::theme::{self, space};

struct MessageApp {
    text: String,
    focus_ok: bool,
}

impl eframe::App for MessageApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut close = ui.input(|i| {
            i.key_pressed(egui::Key::Enter)
                || i.key_pressed(egui::Key::Escape)
                || i.key_pressed(egui::Key::Space)
        });
        let p = theme::palette(ui.ctx());
        let frame = Frame::new()
            .fill(p.bg)
            .inner_margin(Margin::same(space::LG as i8));
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            let button_h = ui.spacing().interact_size.y;
            let text_h = ui.available_height() - button_h - ui.spacing().item_spacing.y;
            egui::ScrollArea::vertical()
                .max_height(text_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.label(self.text.as_str());
                });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let ok = theme::primary_scope(ui, |ui| ui.button("OK"));
                if self.focus_ok {
                    ok.request_focus();
                    self.focus_ok = false;
                }
                close |= ok.clicked();
            });
        });
        if close {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

pub fn run(title: &str, text: &str) -> i32 {
    let viewport = egui::ViewportBuilder::default()
        .with_title(title)
        .with_inner_size([400.0, 170.0])
        .with_resizable(false)
        .with_minimize_button(false)
        .with_maximize_button(false)
        .with_always_on_top();
    let app = MessageApp {
        text: text.to_string(),
        focus_ok: true,
    };
    let result = eframe::run_native(
        "Snipslate",
        super::native_options(viewport),
        Box::new(move |cc| {
            super::theme::install(&cc.egui_ctx);
            super::install_fallback_fonts(&cc.egui_ctx, &format!("{title}{}", app.text));
            Ok(Box::new(app))
        }),
    );
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("snipslate: message window failed: {e}");
            1
        }
    }
}
