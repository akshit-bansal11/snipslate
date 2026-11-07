use std::time::Duration;

const RETRIES: u32 = 5;
const RETRY_DELAY: Duration = Duration::from_millis(100);

/// Copies text, retrying briefly when another app holds the clipboard open.
pub fn set_text(text: &str) -> Result<(), String> {
    write_with_retry(text)?;
    remember(text);
    Ok(())
}

fn write_with_retry(text: &str) -> Result<(), String> {
    // A fresh Clipboard per call: arboard warns against sharing one across threads on Windows.
    let mut attempt = 0;
    loop {
        attempt += 1;
        let result = arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text));
        match result {
            Ok(()) => return Ok(()),
            Err(arboard::Error::ClipboardOccupied) if attempt < RETRIES => {
                std::thread::sleep(RETRY_DELAY);
            }
            Err(arboard::Error::ClipboardOccupied) => {
                return Err("The clipboard is busy. Close the app using it and try again.".into());
            }
            Err(e) => return Err(format!("Could not copy to the clipboard: {e}")),
        }
    }
}

#[cfg(target_os = "linux")]
static LAST: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

#[cfg(target_os = "linux")]
fn remember(text: &str) {
    if let Ok(mut last) = LAST.lock() {
        *last = Some(text.to_owned());
    }
}

#[cfg(not(target_os = "linux"))]
fn remember(_text: &str) {}

/// Linux: hands the last copied text to a windowless holder process, which serves it after this one exits.
// ponytail: one holder process per copy; a newer copy from another run replaces the clipboard owner.
#[cfg(target_os = "linux")]
pub fn finish() {
    let text = match LAST.lock() {
        Ok(last) => last.clone(),
        Err(_) => return,
    };
    if let Some(text) = text
        && let Err(e) = spawn_holder(&text)
    {
        eprintln!("Snipslate: could not keep the copied text on the clipboard: {e}");
    }
}

/// Other OSes keep clipboard data owned by the OS, so nothing to do at exit.
#[cfg(not(target_os = "linux"))]
pub fn finish() {}

#[cfg(target_os = "linux")]
fn spawn_holder(text: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new(std::env::current_exe()?)
        .arg("--hold-clipboard")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    Ok(())
}

/// Linux holder process: serves the clipboard text read from stdin until another app takes it.
#[cfg(target_os = "linux")]
pub fn hold_from_stdin() {
    use arboard::SetExtLinux;
    use std::io::Read;
    let mut text = String::new();
    if std::io::stdin().read_to_string(&mut text).is_err() {
        return;
    }
    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return;
    };
    if let Err(e) = clipboard.set().wait().text(text) {
        eprintln!("Snipslate: clipboard holder stopped: {e}");
    }
}
