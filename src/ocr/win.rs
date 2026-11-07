//! Windows' built-in text recognition. Offline, uses the user's installed language packs.

use crate::imaging;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize};

const NO_LANGUAGE: &str = "Windows has no text-recognition language installed. Add a language in Settings > Time & language > Language & region.";

fn err(e: windows::core::Error) -> String {
    e.message()
}

pub fn read_text(img: &image::RgbaImage) -> Result<String, String> {
    // SAFETY: plain COM/WinRT apartment init on this thread. Failure means it is already
    // initialised (possibly in another mode), which is fine for blocking calls here.
    let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };

    let engine =
        OcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| NO_LANGUAGE.to_string())?;
    let max = OcrEngine::MaxImageDimension().map_err(err)?;
    let img = imaging::fit_long_edge(img, max);

    let (w, h) = (img.width(), img.height());
    let mut bgra = img.as_raw().clone();
    for px in bgra.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
    }

    let writer = DataWriter::new().map_err(err)?;
    writer.WriteBytes(&bgra).map_err(err)?;
    let buffer = writer.DetachBuffer().map_err(err)?;
    let width = i32::try_from(w).map_err(|e| e.to_string())?;
    let height = i32::try_from(h).map_err(|e| e.to_string())?;
    let bitmap =
        SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, width, height)
            .map_err(err)?;

    let result = engine
        .RecognizeAsync(&bitmap)
        .map_err(err)?
        .join()
        .map_err(err)?;
    let mut lines = Vec::new();
    for line in result.Lines().map_err(err)? {
        lines.push(line.Text().map_err(err)?.to_string());
    }
    Ok(lines.join("\n").trim().to_string())
}
