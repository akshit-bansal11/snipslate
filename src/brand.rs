//! The Snipslate logo, drawn in code so the tray process needs no image decoder or GPU.
//!
//! A blue rounded tile with white crop brackets around two text lines; the lower, shorter
//! line is amber: the "translated" line. Edges are anti-aliased by 4x4 supersampling.

/// Tile fill (blue-600).
const TILE: [f32; 3] = [37.0, 99.0, 235.0];
/// Crop brackets and the source text line.
const INK: [f32; 3] = [255.0, 255.0, 255.0];
/// The translated line (amber-400). The logo is the only place this marker colour appears.
const MARKER: [f32; 3] = [251.0, 191.0, 36.0];

/// Samples per pixel along each axis.
const SS: u32 = 4;

/// Signed-distance test for a rounded square centred at 0.5 with half-width `h` and radius `r`.
fn in_rounded_square(x: f32, y: f32, h: f32, r: f32) -> bool {
    let dx = ((x - 0.5).abs() - (h - r)).max(0.0);
    let dy = ((y - 0.5).abs() - (h - r)).max(0.0);
    dx * dx + dy * dy <= r * r
}

/// Four L-shaped corner brackets, built by folding every point into the top-left corner.
fn in_bracket(x: f32, y: f32) -> bool {
    const INSET: f32 = 0.18;
    const THICK: f32 = 0.08;
    const LEN: f32 = 0.22;
    let (fx, fy) = (x.min(1.0 - x), y.min(1.0 - y));
    let along = |v: f32| (INSET..INSET + LEN).contains(&v);
    let across = |v: f32| (INSET..INSET + THICK).contains(&v);
    (along(fx) && across(fy)) || (across(fx) && along(fy))
}

/// A horizontal line with round caps from `x0` to `x1` at height `cy`, half-thickness `r`.
fn in_capsule(x: f32, y: f32, x0: f32, x1: f32, cy: f32, r: f32) -> bool {
    let dx = x - x.clamp(x0, x1);
    let dy = y - cy;
    dx * dx + dy * dy <= r * r
}

/// Colour and coverage of one sample in unit coordinates; `None` is transparent.
fn sample(x: f32, y: f32) -> Option<[f32; 3]> {
    if !in_rounded_square(x, y, 0.47, 0.2) {
        return None;
    }
    if in_bracket(x, y) {
        return Some(INK);
    }
    if in_capsule(x, y, 0.34, 0.66, 0.42, 0.05) {
        return Some(INK);
    }
    if in_capsule(x, y, 0.34, 0.54, 0.6, 0.05) {
        return Some(MARKER);
    }
    Some(TILE)
}

/// Straight-alpha RGBA pixels of the logo at `size` x `size`.
pub fn logo_rgba(size: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    let n = (SS * SS) as f32;
    for py in 0..size {
        for px in 0..size {
            let (mut acc, mut hits) = ([0.0f32; 3], 0u32);
            for sy in 0..SS {
                for sx in 0..SS {
                    let x = (px as f32 + (sx as f32 + 0.5) / SS as f32) / size as f32;
                    let y = (py as f32 + (sy as f32 + 0.5) / SS as f32) / size as f32;
                    if let Some(c) = sample(x, y) {
                        hits += 1;
                        for (a, v) in acc.iter_mut().zip(c) {
                            *a += v;
                        }
                    }
                }
            }
            if hits == 0 {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            // Averaging only covered samples keeps the colour un-premultiplied.
            let h = hits as f32;
            rgba.extend_from_slice(&[
                (acc[0] / h).round() as u8,
                (acc[1] / h).round() as u8,
                (acc[2] / h).round() as u8,
                (h / n * 255.0).round() as u8,
            ]);
        }
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(rgba: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * size + x) * 4) as usize;
        [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
    }

    #[test]
    fn has_the_requested_size() {
        for size in [16, 32, 64] {
            assert_eq!(logo_rgba(size).len(), (size * size * 4) as usize);
        }
    }

    #[test]
    fn corners_are_transparent_and_the_tile_is_opaque() {
        let rgba = logo_rgba(64);
        assert_eq!(px(&rgba, 64, 0, 0)[3], 0);
        assert_eq!(px(&rgba, 64, 63, 63)[3], 0);
        // Between the brackets and the lines: plain tile.
        assert_eq!(px(&rgba, 64, 32, 46), [37, 99, 235, 255]);
    }

    #[test]
    fn brackets_lines_and_marker_are_drawn() {
        let rgba = logo_rgba(64);
        assert_eq!(
            px(&rgba, 64, 14, 12),
            [255, 255, 255, 255],
            "top-left bracket"
        );
        assert_eq!(px(&rgba, 64, 32, 27), [255, 255, 255, 255], "source line");
        assert_eq!(
            px(&rgba, 64, 28, 38),
            [251, 191, 36, 255],
            "translated line"
        );
    }

    #[test]
    fn edges_are_anti_aliased() {
        let rgba = logo_rgba(32);
        let partial = rgba.chunks(4).any(|p| p[3] > 0 && p[3] < 255);
        assert!(partial, "expected partially covered edge pixels");
    }
}
