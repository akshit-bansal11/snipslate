//! macOS' built-in text recognition (Vision). Offline, no language pack to install.

use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use objc2::AllocAnyThread;
use objc2::rc::autoreleasepool;
use objc2_foundation::{NSArray, NSData, NSDictionary};
use objc2_vision::{
    VNImageRequestHandler, VNRecognizeTextRequest, VNRequest, VNRequestTextRecognitionLevel,
};

pub fn read_text(img: &image::RgbaImage) -> Result<String, String> {
    // Full resolution: Vision has no small input cap, and downscaling would cost accuracy.
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;

    autoreleasepool(|_| {
        let data = NSData::with_bytes(&png);
        let handler = VNImageRequestHandler::initWithData_options(
            VNImageRequestHandler::alloc(),
            &data,
            &NSDictionary::new(),
        );

        let request = VNRecognizeTextRequest::new();
        request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
        request.setUsesLanguageCorrection(true);

        let base: &VNRequest = &request;
        handler
            .performRequests_error(&NSArray::from_slice(&[base]))
            .map_err(|e| e.localizedDescription().to_string())?;

        let mut lines = Vec::new();
        if let Some(results) = request.results() {
            for obs in results.iter() {
                if let Some(best) = obs.topCandidates(1).iter().next() {
                    lines.push(best.string().to_string());
                }
            }
        }
        Ok(lines.join("\n").trim().to_string())
    })
}
