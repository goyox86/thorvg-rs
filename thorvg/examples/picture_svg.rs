//! Demonstrates loading and rendering an SVG image via the Picture API.
//!
//! Run with: `cargo run --example picture_svg`
//! Output:   `picture_svg.png`

#![allow(clippy::cast_precision_loss)]

mod common;

use thorvg::{ColorSpace, EngineOption, Rgba, Thorvg};

fn main() {
    let engine = Thorvg::init(0).unwrap();
    let (w, h) = (400u32, 400u32);
    let mut buffer = vec![0u32; (w * h) as usize];
    let mut canvas = engine.sw_canvas(EngineOption::Default).unwrap();
    unsafe { canvas.set_target(&mut buffer, w, w, h, ColorSpace::ABGR8888) }.unwrap();

    // White background
    let mut bg = engine.shape().unwrap();
    bg.append_rect(0.0, 0.0, w as f32, h as f32, 0.0, 0.0, true)
        .unwrap();
    bg.set_fill_color(Rgba::new(255, 255, 255, 255)).unwrap();
    canvas.add(bg).unwrap();

    // Load SVG from embedded string
    let svg_data = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200">"#,
        r#"<defs>"#,
        r#"<linearGradient id="g1" x1="0%" y1="0%" x2="100%" y2="100%">"#,
        r#"<stop offset="0%" style="stop-color:rgb(255,107,107);stop-opacity:1" />"#,
        r#"<stop offset="100%" style="stop-color:rgb(78,205,196);stop-opacity:1" />"#,
        r#"</linearGradient>"#,
        r#"</defs>"#,
        r#"<rect x="10" y="10" width="180" height="180" rx="20" fill="url(#g1)"/>"#,
        r#"<circle cx="70" cy="80" r="25" fill="white" opacity="0.8"/>"#,
        r#"<circle cx="130" cy="80" r="25" fill="white" opacity="0.8"/>"#,
        r#"<path d="M60 130 Q100 170 140 130" stroke="white" stroke-width="5" fill="none" stroke-linecap="round"/>"#,
        r#"</svg>"#,
    );

    let mut pic = engine.picture().unwrap();
    pic.load_data(svg_data.as_bytes(), thorvg::MimeType::Svg, None)
        .unwrap();
    pic.set_size(w as f32, h as f32).unwrap();
    canvas.add(pic).unwrap();

    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
    common::save_png("picture_svg.png", &buffer, w, h);
}
