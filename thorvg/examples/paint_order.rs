//! Demonstrates paint order (stroke-before-fill vs fill-before-stroke)
//! and trim path.
//!
//! Run with: `cargo run --example paint_order`
//! Output:   `paint_order.png`

#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

mod common;

use thorvg::{ColorSpace, EngineOption, StrokeCap, Thorvg};

fn main() {
    let engine = Thorvg::init(0).unwrap();
    let (w, h) = (700u32, 350u32);
    let mut buffer = vec![0u32; (w * h) as usize];
    let mut canvas = engine.sw_canvas(EngineOption::Default).unwrap();
    unsafe { canvas.set_target(&mut buffer, w, w, h, ColorSpace::ABGR8888) }.unwrap();

    // Background
    let mut bg = engine.shape().unwrap();
    bg.append_rect(0.0, 0.0, w as f32, h as f32, 0.0, 0.0, true)
        .unwrap();
    bg.set_fill_color(240, 240, 240, 255).unwrap();
    canvas.push(bg).unwrap();

    // ── Default order: fill first, then stroke ─────────────────────
    let mut s1 = engine.shape().unwrap();
    s1.append_circle(120.0, 120.0, 70.0, 70.0, true).unwrap();
    s1.set_fill_color(100, 150, 255, 255).unwrap();
    s1.set_stroke_width(12.0).unwrap();
    s1.set_stroke_color(255, 80, 80, 255).unwrap();
    // Default: stroke on top of fill
    canvas.push(s1).unwrap();

    // ── Stroke first, then fill (fill covers stroke) ───────────────
    let mut s2 = engine.shape().unwrap();
    s2.append_circle(330.0, 120.0, 70.0, 70.0, true).unwrap();
    s2.set_fill_color(100, 150, 255, 255).unwrap();
    s2.set_stroke_width(12.0).unwrap();
    s2.set_stroke_color(255, 80, 80, 255).unwrap();
    s2.set_paint_order(true).unwrap(); // stroke first
    canvas.push(s2).unwrap();

    // ── Trim path: show partial path ───────────────────────────────
    let mut s3 = engine.shape().unwrap();
    s3.append_circle(540.0, 120.0, 70.0, 70.0, true).unwrap();
    s3.set_stroke_width(6.0).unwrap();
    s3.set_stroke_color(50, 50, 50, 255).unwrap();
    s3.set_stroke_cap(StrokeCap::Round).unwrap();
    s3.set_trimpath(0.0, 0.75, true).unwrap();
    canvas.push(s3).unwrap();

    // ── Multiple trim path values ──────────────────────────────────
    let trims = [0.25, 0.5, 0.75, 1.0];
    for (i, &end) in trims.iter().enumerate() {
        let mut s = engine.shape().unwrap();
        s.append_circle(100.0 + i as f32 * 160.0, 270.0, 40.0, 40.0, true)
            .unwrap();
        s.set_stroke_width(5.0).unwrap();
        let hue = (i * 60 + 50) as u8;
        s.set_stroke_color(hue, 100, 255 - hue, 255).unwrap();
        s.set_stroke_cap(StrokeCap::Round).unwrap();
        s.set_trimpath(0.0, end, true).unwrap();
        canvas.push(s).unwrap();
    }

    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
    common::save_png("paint_order.png", &buffer, w, h);
}
