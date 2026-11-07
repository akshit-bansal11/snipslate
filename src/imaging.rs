use std::borrow::Cow;

use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType, ImageEncoder, RgbaImage};

/// Long edge the vision APIs accept without server-side resizing.
pub const API_MAX_EDGE: u32 = 1568;

/// True when nearly every sampled pixel is black, which is what protected (DRM) video looks like in a capture.
// ponytail: samples a ~100x100 grid; a black terminal with a few words can trip it, so callers offer "Send anyway".
pub fn is_mostly_black(img: &RgbaImage) -> bool {
    let (width, height) = img.dimensions();
    let step_x = usize::max(1, (width / 100) as usize);
    let step_y = usize::max(1, (height / 100) as usize);
    let mut total = 0usize;
    let mut dark = 0usize;
    for y in (0..height as usize).step_by(step_y) {
        for x in (0..width as usize).step_by(step_x) {
            let [r, g, b, _] = img.get_pixel(x as u32, y as u32).0;
            total += 1;
            if u32::from(r) + u32::from(g) + u32::from(b) <= 24 {
                dark += 1;
            }
        }
    }
    total > 0 && dark as f64 >= total as f64 * 0.99
}

/// Returns `img` unchanged when its long edge is already within `max_edge`, otherwise a Lanczos3 resize.
pub fn fit_long_edge(img: &RgbaImage, max_edge: u32) -> Cow<'_, RgbaImage> {
    let (width, height) = img.dimensions();
    let long_edge = width.max(height);
    if long_edge <= max_edge {
        return Cow::Borrowed(img);
    }
    let scale = f64::from(max_edge) / f64::from(long_edge);
    let new_width = u32::max(1, (f64::from(width) * scale).round() as u32);
    let new_height = u32::max(1, (f64::from(height) * scale).round() as u32);
    Cow::Owned(image::imageops::resize(
        img,
        new_width,
        new_height,
        FilterType::Lanczos3,
    ))
}

/// PNG bytes for the API: scaled to `API_MAX_EDGE`, alpha dropped.
pub fn to_png(img: &RgbaImage) -> Result<Vec<u8>, String> {
    let scaled = fit_long_edge(img, API_MAX_EDGE).into_owned();
    let (width, height) = scaled.dimensions();
    let rgb = DynamicImage::ImageRgba8(scaled).into_rgb8();
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(rgb.as_raw(), width, height, ExtendedColorType::Rgb8)
        .map_err(|e| format!("Could not encode the screenshot: {e}"))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);
    const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);

    #[test]
    fn all_black_is_blank() {
        let img = RgbaImage::from_pixel(400, 200, BLACK);
        assert!(is_mostly_black(&img));
    }

    #[test]
    fn black_with_visible_text_area_is_not_blank() {
        let img = RgbaImage::from_fn(400, 200, |_, y| if y < 40 { WHITE } else { BLACK });
        assert!(!is_mostly_black(&img));
    }

    #[test]
    fn dark_theme_is_not_blank() {
        let img = RgbaImage::from_pixel(400, 200, Rgba([30, 30, 30, 255]));
        assert!(!is_mostly_black(&img));
    }

    #[test]
    fn to_png_scales_the_long_edge_down() {
        let img = RgbaImage::from_pixel(3000, 1000, WHITE);
        let decoded = image::load_from_memory(&to_png(&img).unwrap()).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (1568, 523));
    }

    #[test]
    fn to_png_keeps_small_images_as_is() {
        let img = RgbaImage::from_pixel(300, 120, WHITE);
        let decoded = image::load_from_memory(&to_png(&img).unwrap()).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (300, 120));
    }
}
