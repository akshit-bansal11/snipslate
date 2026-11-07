//! Text recognition through the `tesseract` command line tool.

use crate::imaging;
use std::io::{ErrorKind, Write};
use std::process::{Command, Stdio};

pub fn read_text(img: &image::RgbaImage) -> Result<String, String> {
    let png = imaging::to_png(img)?;
    // ponytail: English only (tesseract's default `eng`); upgrade: pass `-l` from a setting.
    let mut child = Command::new("tesseract")
        .args(["stdin", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                "Text recognition needs Tesseract. Install it (for example: sudo apt install tesseract-ocr) and try again.".to_string()
            } else {
                format!("Could not start Tesseract: {e}")
            }
        })?;

    // Write on a thread so a full stdout pipe cannot deadlock against our stdin write.
    let mut stdin = child
        .stdin
        .take()
        .ok_or("Could not open Tesseract's input.")?;
    let writer = std::thread::spawn(move || stdin.write_all(&png));
    let out = child
        .wait_with_output()
        .map_err(|e| format!("Tesseract failed: {e}"))?;
    // A write error is only interesting if tesseract also failed; its stderr explains that.
    let _ = writer.join();

    if !out.status.success() {
        return Err(format!(
            "Tesseract failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
