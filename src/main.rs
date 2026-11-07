#![windows_subsystem = "windows"]

mod clip;
mod hotkey;
mod imaging;
mod ocr;
mod settings;
mod tray;
mod translator;
mod ui;

use std::process::{Child, Command, Stdio};

use ui::snip::Mode;

/// Re-launches this executable with `args`, detached from our stdio.
pub(crate) fn spawn_self(args: &[&str]) -> std::io::Result<Child> {
    Command::new(std::env::current_exe()?)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

/// Shows a message box by spawning the `--message` child.
pub(crate) fn show_message(title: &str, text: &str) {
    if let Err(e) = spawn_self(&["--message", title, text]) {
        eprintln!("snipslate: could not show message \"{title}\": {e}");
    }
}

fn dispatch(args: &[String]) -> i32 {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] => tray::run(),
        ["--translate"] => ui::snip::run(Mode::Translate),
        ["--copy"] => ui::snip::run(Mode::Copy),
        ["--settings"] => ui::settings::run(None),
        ["--settings", hint] => ui::settings::run(Some(hint)),
        ["--message", title, text] => ui::message::run(title, text),
        #[cfg(target_os = "linux")]
        ["--hold-clipboard"] => {
            clip::hold_from_stdin();
            0
        }
        other => {
            show_message("Snipslate", &format!("Unknown option: {}", other.join(" ")));
            2
        }
    }
}

fn main() {
    // The snip child reads the cursor and captures the screen before winit makes the
    // process DPI-aware; an unaware process would get virtualised coordinates.
    #[cfg(windows)]
    {
        use windows::Win32::UI::HiDpi::{
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
        };
        // SAFETY: plain Win32 call with a valid constant, made before any window exists.
        let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = dispatch(&args);
    clip::finish();
    std::process::exit(code);
}
