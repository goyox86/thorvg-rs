//! Demonstrates basic shape drawing: rectangles, circles, and custom paths.
//!
//! Ported from `ThorVG`'s `testShape.cpp` and `testSwCanvas.cpp` examples.
//!
//! Run with: `cargo run --example shapes`
//! Output:   `shapes.png`

#![allow(clippy::cast_precision_loss)]

mod common;

use thorvg::{ColorSpace, EngineOption, Rgba, Circle, Rect, Thorvg};

fn main() {
    let engine = Thorvg::init(0).expect("Failed to initialize ThorVG");

    let width = 800u32;
    let height = 600u32;
    let mut buffer = vec![0u32; (width * height) as usize];

    let mut canvas = engine.sw_canvas(EngineOption::Default).expect("Failed to create canvas");
    unsafe { canvas.set_target(&mut buffer, width, width, height, ColorSpace::ABGR8888) }.unwrap();

    // ── White background ───────────────────────────────────────────
    let mut bg = engine.shape().unwrap();
    bg.append_rect(Rect::new(0.0, 0.0, width as f32, height as f32))
        .unwrap();
    bg.set_fill_color(Rgba::new(255, 255, 255, 255)).unwrap();
    canvas.add(bg).unwrap();

    // ── Red rectangle ──────────────────────────────────────────────
    let mut rect = engine.shape().unwrap();
    rect.append_rect(Rect::new(50.0, 50.0, 200.0, 150.0))
        .unwrap();
    rect.set_fill_color(Rgba::new(255, 0, 0, 255)).unwrap();
    canvas.add(rect).unwrap();

    // ── Rounded green rectangle ────────────────────────────────────
    let mut rounded = engine.shape().unwrap();
    rounded.append_rect(Rect::new(300.0, 50.0, 200.0, 150.0).corner_radius(20.0))
        .unwrap();
    rounded.set_fill_color(Rgba::new(0, 200, 0, 255)).unwrap();
    canvas.add(rounded).unwrap();

    // ── Blue circle ────────────────────────────────────────────────
    let mut circle = engine.shape().unwrap();
    circle.append_circle(Circle::new(150.0, 400.0, 80.0))
        .unwrap();
    circle.set_fill_color(Rgba::new(0, 0, 255, 255)).unwrap();
    canvas.add(circle).unwrap();

    // ── Yellow ellipse ─────────────────────────────────────────────
    let mut ellipse = engine.shape().unwrap();
    ellipse.append_circle(Circle::ellipse(400.0, 400.0, 120.0, 60.0))
        .unwrap();
    ellipse.set_fill_color(Rgba::new(255, 255, 0, 255)).unwrap();
    canvas.add(ellipse).unwrap();

    // ── Orange triangle (custom path) ──────────────────────────────
    let mut triangle = engine.shape().unwrap();
    triangle.move_to(650.0, 50.0).unwrap();
    triangle.line_to(750.0, 200.0).unwrap();
    triangle.line_to(550.0, 200.0).unwrap();
    triangle.close().unwrap();
    triangle.set_fill_color(Rgba::new(255, 128, 0, 255)).unwrap();
    canvas.add(triangle).unwrap();

    // ── Magenta cubic Bézier curve ─────────────────────────────────
    let mut curve = engine.shape().unwrap();
    curve.move_to(550.0, 350.0).unwrap();
    curve
        .cubic_to(600.0, 250.0, 700.0, 550.0, 750.0, 350.0)
        .unwrap();
    curve.set_stroke_width(4.0).unwrap();
    curve.set_stroke_color(Rgba::new(255, 0, 255, 255)).unwrap();
    canvas.add(curve).unwrap();

    // ── Render & save ──────────────────────────────────────────────
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();

    common::save_png("shapes.png", &buffer, width, height);
}
