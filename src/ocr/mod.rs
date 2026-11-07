//! Text recognition. Each OS module exposes `read_text(&RgbaImage) -> Result<String, String>`.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod win;

#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(windows)]
use win as platform;

/// Recognised text, trimmed, lines joined with `\n`. `Ok("")` means no text was found.
pub fn read_text(img: &image::RgbaImage) -> Result<String, String> {
    platform::read_text(img)
}
