//! Shared egui plumbing for the child-process windows (settings, message, snip).

pub mod icons;
pub mod message;
pub mod settings;
pub mod snip;
pub mod theme;

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use eframe::egui;

/// Title-bar and taskbar icon size in pixels.
const WINDOW_ICON: u32 = 64;

/// Window options shared by every Snipslate window: Glow renderer, nothing persisted,
/// app id "snipslate", the logo as window icon, and on macOS no Dock icon (Accessory policy).
/// Each window's creator closure also calls [`theme::install`].
pub fn native_options(viewport: egui::ViewportBuilder) -> eframe::NativeOptions {
    let icon = egui::IconData {
        rgba: crate::brand::logo_rgba(WINDOW_ICON),
        width: WINDOW_ICON,
        height: WINDOW_ICON,
    };
    eframe::NativeOptions {
        viewport: viewport.with_app_id("snipslate").with_icon(icon),
        renderer: eframe::Renderer::Glow,
        persist_window: false,
        #[cfg(target_os = "macos")]
        event_loop_builder: Some(Box::new(|builder| {
            use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
            builder.with_activation_policy(ActivationPolicy::Accessory);
        })),
        ..Default::default()
    }
}

/// One script whose glyphs egui's bundled fonts lack.
struct Group {
    ranges: &'static [(u32, u32)],
    /// `fc-match` language tag (Linux).
    lang: &'static str,
    /// Font file names looked up in the OS font directories (Windows and macOS).
    files: &'static [&'static str],
}

const GROUPS: &[Group] = &[
    Group {
        ranges: &[(0x3400, 0x4DBF), (0x4E00, 0x9FFF)],
        lang: "zh-cn",
        files: &["msyh.ttc", "PingFang.ttc", "Hiragino Sans GB.ttc"],
    },
    Group {
        ranges: &[(0x3040, 0x30FF)],
        lang: "ja",
        files: &["YuGothR.ttc", "meiryo.ttc", "PingFang.ttc"],
    },
    Group {
        ranges: &[(0x1100, 0x11FF), (0xAC00, 0xD7AF)],
        lang: "ko",
        files: &["malgun.ttf", "AppleSDGothicNeo.ttc"],
    },
    Group {
        ranges: &[(0x0600, 0x06FF), (0x0750, 0x077F)],
        lang: "ar",
        files: &[
            "segoeui.ttf",
            "tahoma.ttf",
            "Arial Unicode.ttf",
            "GeezaPro.ttc",
        ],
    },
    Group {
        ranges: &[(0x0900, 0x097F)],
        lang: "hi",
        files: &["Nirmala.ttc", "Devanagari Sangam MN.ttc"],
    },
    Group {
        ranges: &[(0x0980, 0x09FF)],
        lang: "bn",
        files: &["Nirmala.ttc", "Bangla Sangam MN.ttc"],
    },
    Group {
        ranges: &[(0x0E00, 0x0E7F)],
        lang: "th",
        files: &["LeelawUI.ttf", "tahoma.ttf", "Thonburi.ttc"],
    },
];

fn font_dirs() -> &'static [&'static str] {
    if cfg!(windows) {
        &[r"C:\Windows\Fonts"]
    } else {
        &[
            "/System/Library/Fonts",
            "/System/Library/Fonts/Supplemental",
            "/Library/Fonts",
        ]
    }
}

fn find_font(g: &Group) -> Option<PathBuf> {
    if cfg!(target_os = "linux") {
        let out = Command::new("fc-match")
            .args(["-f", "%{file}", &format!(":lang={}", g.lang)])
            .output()
            .ok()?;
        let path = PathBuf::from(String::from_utf8(out.stdout).ok()?.trim());
        return path.is_file().then_some(path);
    }
    font_dirs()
        .iter()
        .flat_map(|dir| g.files.iter().map(move |f| PathBuf::from(dir).join(f)))
        .find(|p| p.is_file())
}

/// egui's bundled fonts lack CJK, Arabic, Indic and Thai. For each script present in
/// `text`, load one system font and append it to every proportional family, on top of
/// [`theme::base_fonts`]. Missing fonts are skipped silently. Call after [`theme::install`]:
/// it rebuilds the font set.
/// ponytail: egui does no text shaping or bidi, so Arabic/Urdu/Indic/Thai render with
/// unjoined glyphs; upgrade path is a shaping-capable renderer. Copy still copies correct text.
pub fn install_fallback_fonts(ctx: &egui::Context, text: &str) {
    let mut fonts = theme::base_fonts();
    let mut added = false;
    for (i, g) in GROUPS.iter().enumerate() {
        let needed = text.chars().any(|c| {
            g.ranges
                .iter()
                .any(|&(lo, hi)| (lo..=hi).contains(&(c as u32)))
        });
        if !needed {
            continue;
        }
        let Some(bytes) = find_font(g).and_then(|p| std::fs::read(p).ok()) else {
            continue;
        };
        let name = format!("snipslate-fallback-{i}");
        fonts
            .font_data
            .insert(name.clone(), Arc::new(egui::FontData::from_owned(bytes)));
        for (family, list) in fonts.families.iter_mut() {
            if *family != egui::FontFamily::Monospace {
                list.push(name.clone());
            }
        }
        added = true;
    }
    if added {
        ctx.set_fonts(fonts);
    }
}
