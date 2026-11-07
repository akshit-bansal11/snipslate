//! Tray icon, menu and global hotkeys. Every action launches a child process of this
//! executable (see `main.rs`); the tray itself never opens a window.

use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::hotkey::{Hotkey, hotkeys_supported};
use crate::settings::{self, AppSettings};
use crate::{show_message, spawn_self};

/// Messages posted to the loop thread by waiter threads and menu handlers.
enum Wake {
    Reload,
    Quit,
}

/// Posts a [`Wake`] to whichever event loop this OS runs.
type Waker = Arc<dyn Fn(Wake) + Send + Sync>;

const SIZE: u32 = 32;

/// 32x32 RGBA: accent `#2563eb` rounded square with white corner brackets.
fn icon_rgba() -> Vec<u8> {
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            // Distance from the pixel centre to the inner rectangle shrunk by the radius.
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let (dx, dy) = (fx - fx.clamp(6.0, 26.0), fy - fy.clamp(6.0, 26.0));
            if dx * dx + dy * dy > 36.0 {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            // Fold to the top-left corner: brackets are 2 px thick, 7 px long, inset 7 px.
            let (mx, my) = (x.min(SIZE - 1 - x), y.min(SIZE - 1 - y));
            let bracket = ((7..14).contains(&mx) && (7..9).contains(&my))
                || ((7..9).contains(&mx) && (7..14).contains(&my));
            let (r, g, b) = if bracket {
                (255, 255, 255)
            } else {
                (0x25, 0x63, 0xeb)
            };
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    rgba
}

/// Spawns `args` as a child; when it ends, reports a failure and asks the tray to reload settings.
fn launch(args: &[&str], waker: &Waker) {
    match spawn_self(args) {
        Ok(mut child) => {
            let waker = Arc::clone(waker);
            thread::spawn(move || {
                match child.wait() {
                    Ok(status) if !status.success() => {
                        let code = status
                            .code()
                            .map_or("unknown".to_owned(), |c| c.to_string());
                        show_message(
                            "Snipslate",
                            &format!("Something went wrong (exit code {code})."),
                        );
                    }
                    Ok(_) => {}
                    Err(e) => eprintln!("snipslate: waiting for a child failed: {e}"),
                }
                waker(Wake::Reload);
            });
        }
        Err(e) => show_message("Snipslate", &format!("Could not start: {e}")),
    }
}

const TRANSLATE: &[&str] = &["--translate"];
const COPY: &[&str] = &["--copy"];
const SETTINGS: &[&str] = &["--settings"];

/// Registered `HotKey::id()` values; a reload swaps them under the event handler.
type Ids = Arc<Mutex<(Option<u32>, Option<u32>)>>;

struct Tray {
    _icon: TrayIcon,
    translate_item: MenuItem,
    copy_item: MenuItem,
    manager: Option<GlobalHotKeyManager>,
    registered: [Option<HotKey>; 2],
    /// The hotkey strings currently applied (translate, copy).
    keys: (String, String),
    ids: Ids,
}

impl Tray {
    fn new(waker: Waker) -> Result<Tray, String> {
        let translate_item = MenuItem::new("Translate an area", true, None);
        let copy_item = MenuItem::new("Copy text from an area", true, None);
        let settings_item = MenuItem::new("Settings\u{2026}", true, None);
        let quit_item = MenuItem::new("Quit", true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &translate_item,
            &copy_item,
            &PredefinedMenuItem::separator(),
            &settings_item,
            &quit_item,
        ])
        .map_err(|e| e.to_string())?;
        let icon = Icon::from_rgba(icon_rgba(), SIZE, SIZE).map_err(|e| e.to_string())?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("Snipslate")
            .with_icon(icon)
            .build()
            .map_err(|e| e.to_string())?;

        let (translate_id, copy_id): (MenuId, MenuId) =
            (translate_item.id().clone(), copy_item.id().clone());
        let (settings_id, quit_id) = (settings_item.id().clone(), quit_item.id().clone());
        let menu_waker = Arc::clone(&waker);
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let id = event.id();
            if *id == translate_id {
                launch(TRANSLATE, &menu_waker);
            } else if *id == copy_id {
                launch(COPY, &menu_waker);
            } else if *id == settings_id {
                launch(SETTINGS, &menu_waker);
            } else if *id == quit_id {
                menu_waker(Wake::Quit);
            }
        }));

        let click_waker = Arc::clone(&waker);
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            // Left click translates, as the C# tray did on double click. Windows and
            // macOS only; the Linux indicator backend sends no click events.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                launch(TRANSLATE, &click_waker);
            }
        }));

        let ids: Ids = Arc::default();
        let hotkey_ids = Arc::clone(&ids);
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state != HotKeyState::Pressed {
                return;
            }
            let (translate, copy) = *hotkey_ids.lock().unwrap_or_else(PoisonError::into_inner);
            if Some(event.id) == translate {
                launch(TRANSLATE, &waker);
            } else if Some(event.id) == copy {
                launch(COPY, &waker);
            }
        }));

        let manager = if hotkeys_supported() {
            GlobalHotKeyManager::new()
                .inspect_err(|e| eprintln!("snipslate: global hotkeys unavailable: {e}"))
                .ok()
        } else {
            None
        };
        let mut this = Tray {
            _icon: tray,
            translate_item,
            copy_item,
            manager,
            registered: [None, None],
            keys: (String::new(), String::new()),
            ids,
        };
        this.apply(&AppSettings::load());
        Ok(this)
    }

    /// Re-reads settings; touches hotkeys and menu text only when a hotkey string changed.
    fn reload(&mut self) {
        let settings = AppSettings::load();
        if settings.translate_hotkey != self.keys.0 || settings.copy_hotkey != self.keys.1 {
            self.apply(&settings);
        }
    }

    fn apply(&mut self, settings: &AppSettings) {
        self.keys = (
            settings.translate_hotkey.clone(),
            settings.copy_hotkey.clone(),
        );
        let label = |name: &str, keys: &str| {
            if hotkeys_supported() {
                format!("{name}  ({keys})")
            } else {
                name.to_owned()
            }
        };
        self.translate_item
            .set_text(label("Translate an area", &self.keys.0));
        self.copy_item
            .set_text(label("Copy text from an area", &self.keys.1));
        if !hotkeys_supported() {
            return;
        }

        let old = std::mem::take(&mut self.registered);
        if let Some(manager) = &self.manager {
            for hk in old.into_iter().flatten() {
                if let Err(e) = manager.unregister(hk) {
                    eprintln!("snipslate: could not unregister a hotkey: {e}");
                }
            }
        }
        let texts = [self.keys.0.clone(), self.keys.1.clone()];
        let mut failed = Vec::new();
        for (slot, text) in texts.iter().enumerate() {
            self.registered[slot] = self.register(text);
            if self.registered[slot].is_none() {
                failed.push(text.as_str());
            }
        }
        // ponytail: a HotKey id is a hash, so two different keys could collide; ignored (2^-32).
        *self.ids.lock().unwrap_or_else(PoisonError::into_inner) = (
            self.registered[0].map(|h| h.id()),
            self.registered[1].map(|h| h.id()),
        );
        if !failed.is_empty() {
            show_message(
                "Hotkey unavailable",
                &format!(
                    "{} is already used by another app. Pick another in Settings; the tray menu still works.",
                    failed.join(" and ")
                ),
            );
        }
    }

    /// `None` when the text does not parse or the OS refuses the combination.
    fn register(&self, text: &str) -> Option<HotKey> {
        let hotkey = Hotkey::parse(text)?.to_hot_key();
        self.manager.as_ref()?.register(hotkey).ok()?;
        Some(hotkey)
    }
}

/// Windows and macOS: a window-less winit loop that sleeps until an event arrives.
#[cfg(any(windows, target_os = "macos"))]
fn event_loop() -> i32 {
    use winit::application::ApplicationHandler;
    use winit::event::{StartCause, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::window::WindowId;

    struct App {
        waker: Waker,
        tray: Option<Tray>,
        failed: bool,
    }

    impl ApplicationHandler<Wake> for App {
        // macOS needs the tray created after the loop has started.
        fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
            if !matches!(cause, StartCause::Init) {
                return;
            }
            match Tray::new(Arc::clone(&self.waker)) {
                Ok(tray) => self.tray = Some(tray),
                Err(e) => {
                    show_message("Snipslate", &format!("Could not create the tray icon: {e}"));
                    self.failed = true;
                    event_loop.exit();
                }
            }
        }

        fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

        fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}

        fn user_event(&mut self, event_loop: &ActiveEventLoop, event: Wake) {
            match event {
                Wake::Reload => {
                    if let Some(tray) = &mut self.tray {
                        tray.reload();
                    }
                }
                Wake::Quit => event_loop.exit(),
            }
        }
    }

    #[cfg(target_os = "macos")]
    let built = {
        use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
        let mut builder = EventLoop::<Wake>::with_user_event();
        builder.with_activation_policy(ActivationPolicy::Accessory);
        builder.build()
    };
    #[cfg(not(target_os = "macos"))]
    let built = EventLoop::<Wake>::with_user_event().build();
    let event_loop = match built {
        Ok(event_loop) => event_loop,
        Err(e) => {
            show_message("Snipslate", &format!("Could not start the event loop: {e}"));
            return 1;
        }
    };
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();
    let mut app = App {
        waker: Arc::new(move |wake| {
            let _ = proxy.send_event(wake);
        }),
        tray: None,
        failed: false,
    };
    let result = event_loop.run_app(&mut app);
    let failed = app.failed;
    drop(app); // drops the tray icon so no ghost icon is left behind
    match result {
        Ok(()) if !failed => 0,
        Ok(()) => 1,
        Err(e) => {
            show_message("Snipslate", &format!("The tray event loop failed: {e}"));
            1
        }
    }
}

/// Linux: the GTK main loop on the main thread; waiter threads post over a channel.
#[cfg(target_os = "linux")]
fn event_loop() -> i32 {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;
    use std::time::Duration;

    use gtk::glib::{ControlFlow, timeout_add_local};

    if let Err(e) = gtk::init() {
        show_message("Snipslate", &format!("Could not initialise GTK: {e}"));
        return 1;
    }
    let (tx, rx) = mpsc::channel::<Wake>();
    let waker: Waker = Arc::new(move |wake| {
        let _ = tx.send(wake);
    });
    let tray = match Tray::new(waker) {
        Ok(tray) => Rc::new(RefCell::new(tray)),
        Err(e) => {
            show_message("Snipslate", &format!("Could not create the tray icon: {e}"));
            return 1;
        }
    };
    // ponytail: 500 ms polling on Linux only (Windows/macOS sleep); upgrade to
    // glib::MainContext::invoke from the waiter threads if the wake-up latency matters.
    let in_loop = Rc::clone(&tray);
    timeout_add_local(Duration::from_millis(500), move || {
        while let Ok(wake) = rx.try_recv() {
            match wake {
                Wake::Reload => in_loop.borrow_mut().reload(),
                Wake::Quit => {
                    gtk::main_quit();
                    return ControlFlow::Break;
                }
            }
        }
        ControlFlow::Continue
    });
    gtk::main();
    drop(tray); // drops the tray icon so no ghost icon is left behind
    0
}

/// Runs the tray until Quit. Returns the process exit code.
pub fn run() -> i32 {
    // Held for the whole run; dropping it releases the lock.
    let _lock = match settings::try_lock("tray.lock") {
        Ok(Some(file)) => Some(file),
        Ok(None) => {
            show_message(
                "Snipslate",
                "Snipslate is already running. Look for its icon in the system tray.",
            );
            return 0;
        }
        Err(e) => {
            // ponytail: without a lock a second copy can start; acceptable when the config dir is unwritable.
            eprintln!("snipslate: could not take the single-instance lock: {e}");
            None
        }
    };
    event_loop()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_is_32x32_rgba_with_opaque_centre() {
        let rgba = icon_rgba();
        assert_eq!(rgba.len(), 32 * 32 * 4);
        assert_eq!(rgba[(16 * 32 + 16) * 4 + 3], 255);
        assert_eq!(rgba[3], 0, "corner pixel is transparent");
    }
}
