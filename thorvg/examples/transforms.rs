//! Demonstrates paint transformations: translate, rotate, scale, and matrix transforms.
//!
//! Run with: `cargo run --example transforms`
//! Output:   `transforms.png`

#![allow(clippy::cast_precision_loss)]

mod common;

use thorvg::{ColorSpace, EngineOption, Matrix, Paint, Shape, SwCanvas, Thorvg};

fn main() {
    let _engine = Thorvg::init(0).unwrap();
    let (w, h) = (800u32, 400u32);
    let mut buffer = vec![0u32; (w * h) as usize];
    let mut canvas = SwCanvas::new(EngineOption::Default).unwrap();
    canvas.set_target(&mut buffer, w, w, h, ColorSpace::ABGR8888).unwrap();

    // Background
    let mut bg = Shape::new();
    bg.append_rect(0.0, 0.0, w as f32, h as f32, 0.0, 0.0, true).unwrap();
    bg.set_fill_color(30, 30, 40, 255).unwrap();
    canvas.push(bg).unwrap();

    // ── Original (no transform) ────────────────────────────────────
    let mut s1 = Shape::new();
    s1.append_rect(0.0, 0.0, 60.0, 60.0, 5.0, 5.0, true).unwrap();
    s1.set_fill_color(255, 80, 80, 255).unwrap();
    s1.translate(50.0, 50.0).unwrap();
    canvas.push(s1).unwrap();

    // ── Translated ─────────────────────────────────────────────────
    let mut s2 = Shape::new();
    s2.append_rect(0.0, 0.0, 60.0, 60.0, 5.0, 5.0, true).unwrap();
    s2.set_fill_color(80, 255, 80, 255).unwrap();
    s2.translate(200.0, 50.0).unwrap();
    canvas.push(s2).unwrap();

    // ── Rotated 30° ────────────────────────────────────────────────
    let mut s3 = Shape::new();
    s3.append_rect(0.0, 0.0, 60.0, 60.0, 5.0, 5.0, true).unwrap();
    s3.set_fill_color(80, 80, 255, 255).unwrap();
    s3.translate(350.0, 80.0).unwrap();
    s3.rotate(30.0).unwrap();
    canvas.push(s3).unwrap();

    // ── Scaled 1.5x ────────────────────────────────────────────────
    let mut s4 = Shape::new();
    s4.append_rect(0.0, 0.0, 60.0, 60.0, 5.0, 5.0, true).unwrap();
    s4.set_fill_color(255, 255, 80, 255).unwrap();
    s4.translate(500.0, 50.0).unwrap();
    s4.scale(1.5).unwrap();
    canvas.push(s4).unwrap();

    // ── Custom matrix: skew transform ──────────────────────────────
    let mut s5 = Shape::new();
    s5.append_rect(0.0, 0.0, 60.0, 60.0, 5.0, 5.0, true).unwrap();
    s5.set_fill_color(255, 80, 255, 255).unwrap();
    s5.set_transform(&Matrix {
        e11: 1.0,  e12: 0.3, e13: 680.0,
        e21: 0.0,  e22: 1.0, e23: 50.0,
        e31: 0.0,  e32: 0.0, e33: 1.0,
    }).unwrap();
    canvas.push(s5).unwrap();

    // ── Row of rotated shapes ──────────────────────────────────────
    for i in 0..8 {
        let mut s = Shape::new();
        s.append_rect(0.0, 0.0, 30.0, 30.0, 3.0, 3.0, true).unwrap();
        let hue = (i * 32) as u8;
        s.set_fill_color(255, hue, 255 - hue, 200).unwrap();
        s.translate(80.0 + i as f32 * 85.0, 280.0).unwrap();
        s.rotate(i as f32 * 15.0).unwrap();
        canvas.push(s).unwrap();
    }

    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
    common::save_png("transforms.png", &buffer, w, h);
}
