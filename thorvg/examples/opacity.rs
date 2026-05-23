//! Demonstrates opacity and visibility controls.
//!
//! Run with: `cargo run --example opacity`
//! Output:   `opacity.png`

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

mod common;

use thorvg::{ColorSpace, EngineOption, Paint, Shape, SwCanvas, Thorvg};

fn main() {
    let _engine = Thorvg::init(0).unwrap();
    let (w, h) = (800u32, 300u32);
    let mut buffer = vec![0u32; (w * h) as usize];
    let mut canvas = SwCanvas::new(EngineOption::Default).unwrap();
    unsafe { canvas.set_target(&mut buffer, w, w, h, ColorSpace::ABGR8888) }.unwrap();

    // Checkerboard background to show transparency
    for row in 0..(h / 20) {
        for col in 0..(w / 20) {
            let mut sq = Shape::new();
            sq.append_rect(
                col as f32 * 20.0,
                row as f32 * 20.0,
                20.0,
                20.0,
                0.0,
                0.0,
                true,
            )
            .unwrap();
            let gray = if (row + col) % 2 == 0 { 200 } else { 230 };
            sq.set_fill_color(gray, gray, gray, 255).unwrap();
            canvas.push(sq).unwrap();
        }
    }

    // Row of circles with decreasing opacity
    for i in 0..10 {
        let mut circle = Shape::new();
        circle
            .append_circle(60.0 + i as f32 * 75.0, 150.0, 30.0, 30.0, true)
            .unwrap();
        circle.set_fill_color(0, 100, 255, 255).unwrap();

        let opacity = 255 - (i * 25) as u8;
        circle.set_opacity(opacity).unwrap();
        canvas.push(circle).unwrap();
    }

    // Overlapping semi-transparent rectangles
    let colors: &[(u8, u8, u8)] = &[(255, 0, 0), (0, 255, 0), (0, 0, 255)];
    for (i, &(r, g, b)) in colors.iter().enumerate() {
        let mut rect = Shape::new();
        rect.append_rect(150.0 + i as f32 * 60.0, 50.0, 120.0, 80.0, 10.0, 10.0, true)
            .unwrap();
        rect.set_fill_color(r, g, b, 150).unwrap();
        canvas.push(rect).unwrap();
    }

    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
    common::save_png("opacity.png", &buffer, w, h);
}
