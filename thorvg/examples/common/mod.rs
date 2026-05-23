use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// Save an ABGR8888 buffer as a PNG file.
///
/// Each `u32` pixel is laid out as `[R, G, B, A]` from LSB to MSB in ABGR8888:
/// - bits  0–7:  R
/// - bits  8–15: G
/// - bits 16–23: B
/// - bits 24–31: A
pub fn save_png(path: &str, buffer: &[u32], width: u32, height: u32) {
    let file = File::create(Path::new(path)).expect("Failed to create output file");
    let w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().expect("Failed to write PNG header");

    // Convert ABGR8888 → RGBA bytes
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for &pixel in buffer {
        let r = (pixel & 0xFF) as u8;
        let g = ((pixel >> 8) & 0xFF) as u8;
        let b = ((pixel >> 16) & 0xFF) as u8;
        let a = ((pixel >> 24) & 0xFF) as u8;
        rgba.extend_from_slice(&[r, g, b, a]);
    }

    writer
        .write_image_data(&rgba)
        .expect("Failed to write PNG data");

    println!("Saved {width}x{height} → {path}");
}
