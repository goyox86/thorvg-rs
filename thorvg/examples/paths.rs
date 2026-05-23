//! Demonstrates complex path construction: Bézier curves, stars, spirals, and custom shapes.
//!
//! Run with: `cargo run --example paths`
//! Output:   `paths.png`

#![allow(clippy::cast_precision_loss)]

mod common;

use thorvg::{ColorSpace, EngineOption, Shape, StrokeCap, StrokeJoin, SwCanvas, Thorvg};

fn main() {
    let _engine = Thorvg::init(0).unwrap();
    let (w, h) = (800u32, 600u32);
    let mut buffer = vec![0u32; (w * h) as usize];
    let mut canvas = SwCanvas::new(EngineOption::Default).unwrap();
    canvas.set_target(&mut buffer, w, w, h, ColorSpace::ABGR8888).unwrap();

    // Background
    let mut bg = Shape::new();
    bg.append_rect(0.0, 0.0, w as f32, h as f32, 0.0, 0.0, true).unwrap();
    bg.set_fill_color(15, 15, 25, 255).unwrap();
    canvas.push(bg).unwrap();

    // ── 5-pointed star ─────────────────────────────────────────────
    let mut star = Shape::new();
    let (cx, cy) = (120.0f32, 140.0f32);
    let (outer, inner) = (80.0f32, 35.0f32);
    for i in 0..10 {
        let angle = core::f32::consts::PI * i as f32 / 5.0 - core::f32::consts::FRAC_PI_2;
        let r = if i % 2 == 0 { outer } else { inner };
        let px = cx + angle.cos() * r;
        let py = cy + angle.sin() * r;
        if i == 0 { star.move_to(px, py).unwrap(); }
        else { star.line_to(px, py).unwrap(); }
    }
    star.close().unwrap();
    star.set_fill_color(255, 200, 0, 255).unwrap();
    star.set_stroke_width(2.0).unwrap();
    star.set_stroke_color(255, 255, 200, 255).unwrap();
    canvas.push(star).unwrap();

    // ── Heart shape (cubic Béziers) ────────────────────────────────
    let mut heart = Shape::new();
    let hx = 320.0f32;
    let hy = 160.0f32;
    heart.move_to(hx, hy).unwrap();
    heart.cubic_to(hx - 5.0, hy - 60.0, hx - 100.0, hy - 60.0, hx - 100.0, hy - 10.0).unwrap();
    heart.cubic_to(hx - 100.0, hy + 30.0, hx, hy + 80.0, hx, hy + 100.0).unwrap();
    heart.cubic_to(hx, hy + 80.0, hx + 100.0, hy + 30.0, hx + 100.0, hy - 10.0).unwrap();
    heart.cubic_to(hx + 100.0, hy - 60.0, hx + 5.0, hy - 60.0, hx, hy).unwrap();
    heart.close().unwrap();
    heart.set_fill_color(220, 30, 60, 255).unwrap();
    canvas.push(heart).unwrap();

    // ── Spiral (stroked path) ──────────────────────────────────────
    let mut spiral = Shape::new();
    let (sx, sy) = (580.0f32, 140.0f32);
    spiral.move_to(sx, sy).unwrap();
    let turns = 4.0f32;
    let steps = 200;
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let angle = turns * 2.0 * core::f32::consts::PI * t;
        let r = 5.0 + 80.0 * t;
        let px = sx + angle.cos() * r;
        let py = sy + angle.sin() * r;
        spiral.line_to(px, py).unwrap();
    }
    spiral.set_stroke_width(2.5).unwrap();
    spiral.set_stroke_color(100, 200, 255, 255).unwrap();
    spiral.set_stroke_cap(StrokeCap::Round).unwrap();
    canvas.push(spiral).unwrap();

    // ── Wavy line ──────────────────────────────────────────────────
    let mut wave = Shape::new();
    wave.move_to(50.0, 420.0).unwrap();
    let segments = 12;
    let seg_w = 60.0f32;
    for i in 0..segments {
        let x0 = 50.0 + i as f32 * seg_w;
        let dir = if i % 2 == 0 { -60.0 } else { 60.0 };
        wave.cubic_to(
            x0 + seg_w * 0.33, 420.0 + dir,
            x0 + seg_w * 0.66, 420.0 + dir,
            x0 + seg_w, 420.0,
        ).unwrap();
    }
    wave.set_stroke_width(4.0).unwrap();
    wave.set_stroke_color(255, 150, 50, 255).unwrap();
    wave.set_stroke_cap(StrokeCap::Round).unwrap();
    wave.set_stroke_join(StrokeJoin::Round).unwrap();
    canvas.push(wave).unwrap();

    // ── Polygon (hexagon) ──────────────────────────────────────────
    let mut hex = Shape::new();
    let (hcx, hcy, hr) = (150.0f32, 520.0f32, 50.0f32);
    for i in 0..6 {
        let angle = core::f32::consts::PI * i as f32 / 3.0 - core::f32::consts::FRAC_PI_2;
        let px = hcx + angle.cos() * hr;
        let py = hcy + angle.sin() * hr;
        if i == 0 { hex.move_to(px, py).unwrap(); }
        else { hex.line_to(px, py).unwrap(); }
    }
    hex.close().unwrap();
    hex.set_fill_color(60, 180, 120, 200).unwrap();
    hex.set_stroke_width(3.0).unwrap();
    hex.set_stroke_color(200, 255, 200, 255).unwrap();
    canvas.push(hex).unwrap();

    // ── Arrow shape ────────────────────────────────────────────────
    let mut arrow = Shape::new();
    arrow.move_to(350.0, 500.0).unwrap();
    arrow.line_to(500.0, 530.0).unwrap();
    arrow.line_to(350.0, 560.0).unwrap();
    arrow.line_to(380.0, 530.0).unwrap();
    arrow.close().unwrap();
    arrow.set_fill_color(255, 100, 200, 255).unwrap();
    canvas.push(arrow).unwrap();

    // ── Infinity symbol (figure-8 with Béziers) ────────────────────
    let mut inf = Shape::new();
    let ix = 620.0f32;
    let iy = 530.0f32;
    inf.move_to(ix, iy).unwrap();
    inf.cubic_to(ix + 70.0, iy - 70.0, ix + 130.0, iy + 70.0, ix + 70.0, iy).unwrap();
    inf.cubic_to(ix + 10.0, iy - 70.0, ix - 60.0, iy + 70.0, ix, iy).unwrap();
    inf.set_stroke_width(4.0).unwrap();
    inf.set_stroke_color(200, 200, 255, 255).unwrap();
    inf.set_stroke_cap(StrokeCap::Round).unwrap();
    canvas.push(inf).unwrap();

    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
    common::save_png("paths.png", &buffer, w, h);
}
