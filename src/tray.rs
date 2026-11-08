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
    /// Settings opened: release the global hotkeys so the recorder sees the keypresses.
    Pause,
    /// Settings closed: register the saved hotkeys again.
    Resume,
    Reload,
    Quit,
}

/// Posts a [`Wake`] to whichever event loop this OS runs.
type Waker = Arc<dyn Fn(Wake) + Send + Sync>;

/// Tray icon size in pixels; the OS scales it to 16-24 px.
const SIZE: u32 = 32;

/// Spawns `args` as a child; when it ends, reports a failure and asks the tray to reload settings.
fn launch(args: &[&str], waker: &Waker) {
    match spawn_self(args) {
        Ok(mut child) => {
            let waker = Arc::clone(waker);
            let resume = args == SETTINGS;
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
                waker(if resume { Wake::Resume } else { Wake::Reload });
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
    /// True while Settings is open; hotkeys stay unregistered.
    paused: bool,
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
        let icon = Icon::from_rgba(crate::brand::logo_rgba(SIZE), SIZE, SIZE)
            .map_err(|e| e.to_string())?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            // macOS convention: a left click opens the menu; elsewhere it translates (below).
            .with_menu_on_left_click(cfg!(target_os = "macos"))
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
                // ponytail: Settings opened by a snip child (missing-key hint path) does not pause the
                // hotkeys; recording there can trigger a snip. Route that launch through the tray to fix.
                menu_waker(Wake::Pause);
                launch(SETTINGS, &menu_waker);
            } else if *id == quit_id {
                menu_waker(Wake::Quit);
            }
        }));

        let click_waker = Arc::clone(&waker);
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            // Left click translates on Windows, as the C# tray did on double click. macOS opens
            // the menu instead; the Linux indicator backend sends no click events.
            if cfg!(windows)
                && let TrayIconEvent::Click {
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
            paused: false,
        };
        this.apply(&AppSettings::load());
        Ok(this)
    }

    /// Re-reads settings; touches hotkeys and menu text only when a hotkey string changed.
    fn reload(&mut self) {
        if self.paused {
            return;
        }
        let settings = AppSettings::load();
        if settings.translate_hotkey != self.keys.0 || settings.copy_hotkey != self.keys.1 {
            self.apply(&settings);
        }
    }

    fn pause(&mut self) {
        self.paused = true;
        if let Some(manager) = &self.manager {
            for hk in std::mem::take(&mut self.registered).into_iter().flatten() {
                if let Err(e) = manager.unregister(hk) {
                    eprintln!("snipslate: could not unregister a hotkey: {e}");
                }
            }
        }
        *self.ids.lock().unwrap_or_else(PoisonError::into_inner) = (None, None);
    }

    /// Re-applies the saved hotkeys even when the strings are unchanged (`pause` dropped them).
    fn resume(&mut self) {
        self.paused = false;
        self.apply(&AppSettings::load());
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
        let (mut invalid, mut failed) = (Vec::new(), Vec::new());
        for (slot, text) in texts.iter().enumerate() {
            match Hotkey::parse(text) {
                None => invalid.push(text.as_str()),
                Some(parsed) => {
                    self.registered[slot] = self.register(parsed.to_hot_key());
                    if self.registered[slot].is_none() {
                        failed.push(text.as_str());
                    }
                }
            }
        }
        // ponytail: a HotKey id is a hash, so two different keys could collide; ignored (2^-32).
        *self.ids.lock().unwrap_or_else(PoisonError::into_inner) = (
            self.registered[0].map(|h| h.id()),
            self.registered[1].map(|h| h.id()),
        );
        for text in invalid {
            show_message(
                "Hotkey unavailable",
                &format!(
                    "\"{text}\" isn't a valid hotkey. Pick another in Settings; the tray menu still works."
                ),
            );
        }
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

    /// `None` when the OS refuses the combination.
    fn register(&self, hotkey: HotKey) -> Option<HotKey> {
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
                Wake::Pause => {
                    if let Some(tray) = &mut self.tray {
                        tray.pause();
                    }
                }
                Wake::Resume => {
                    if let Some(tray) = &mut self.tray {
                        tray.resume();
                    }
                }
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
                Wake::Pause => in_loop.borrow_mut().pause(),
                Wake::Resume => in_loop.borrow_mut().resume(),
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
