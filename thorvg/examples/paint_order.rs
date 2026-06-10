//! Demonstrates paint order (stroke-before-fill vs fill-before-stroke)
//! and trim path.
//!
//! Run with: `cargo run --example paint_order`
//! Output:   `paint_order.png`

#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

mod common;

use thorvg::{ColorSpace, EngineOption, StrokeCap, Rgba, Circle, Rect, Thorvg};

fn main() {
    let engine = Thorvg::init(0).unwrap();
    let (w, h) = (700u32, 350u32);
    let mut buffer = vec![0u32; (w * h) as usize];
    let mut canvas = engine.sw_canvas(EngineOption::Default).unwrap();
    unsafe { canvas.set_target(&mut buffer, w, w, h, ColorSpace::ABGR8888) }.unwrap();

    // Background
    let mut bg = engine.shape().unwrap();
    bg.append_rect(Rect { x: 0.0, y: 0.0, width: w as f32, height: h as f32, rx: 0.0, ry: 0.0, cw: true })
        .unwrap();
    bg.set_fill_color(Rgba::new(240, 240, 240, 255)).unwrap();
    canvas.add(bg).unwrap();

    // ── Default order: fill first, then stroke ─────────────────────
    let mut s1 = engine.shape().unwrap();
    s1.append_circle(Circle { cx: 120.0, cy: 120.0, rx: 70.0, ry: 70.0, cw: true }).unwrap();
    s1.set_fill_color(Rgba::new(100, 150, 255, 255)).unwrap();
    s1.set_stroke_width(12.0).unwrap();
    s1.set_stroke_color(Rgba::new(255, 80, 80, 255)).unwrap();
    // Default: stroke on top of fill
    canvas.add(s1).unwrap();

    // ── Stroke first, then fill (fill covers stroke) ───────────────
    let mut s2 = engine.shape().unwrap();
    s2.append_circle(Circle { cx: 330.0, cy: 120.0, rx: 70.0, ry: 70.0, cw: true }).unwrap();
    s2.set_fill_color(Rgba::new(100, 150, 255, 255)).unwrap();
    s2.set_stroke_width(12.0).unwrap();
    s2.set_stroke_color(Rgba::new(255, 80, 80, 255)).unwrap();
    s2.set_paint_order(true).unwrap(); // stroke first
    canvas.add(s2).unwrap();

    // ── Trim path: show partial path ───────────────────────────────
    let mut s3 = engine.shape().unwrap();
    s3.append_circle(Circle { cx: 540.0, cy: 120.0, rx: 70.0, ry: 70.0, cw: true }).unwrap();
    s3.set_stroke_width(6.0).unwrap();
    s3.set_stroke_color(Rgba::new(50, 50, 50, 255)).unwrap();
    s3.set_stroke_cap(StrokeCap::Round).unwrap();
    s3.set_trimpath(0.0, 0.75, true).unwrap();
    canvas.add(s3).unwrap();

    // ── Multiple trim path values ──────────────────────────────────
    let trims = [0.25, 0.5, 0.75, 1.0];
    for (i, &end) in trims.iter().enumerate() {
        let mut s = engine.shape().unwrap();
        s.append_circle(Circle { cx: 100.0 + i as f32 * 160.0, cy: 270.0, rx: 40.0, ry: 40.0, cw: true })
            .unwrap();
        s.set_stroke_width(5.0).unwrap();
        let hue = (i * 60 + 50) as u8;
        s.set_stroke_color(Rgba::new(hue, 100, 255 - hue, 255)).unwrap();
        s.set_stroke_cap(StrokeCap::Round).unwrap();
        s.set_trimpath(0.0, end, true).unwrap();
        canvas.add(s).unwrap();
    }

    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
    common::save_png("paint_order.png", &buffer, w, h);
}
