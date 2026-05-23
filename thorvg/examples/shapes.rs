//! Demonstrates basic shape drawing: rectangles, circles, and custom paths.
//!
//! Ported from `ThorVG`'s `testShape.cpp` and `testSwCanvas.cpp` examples.
//!
//! Run with: `cargo run --example shapes`
//! Output:   `shapes.png`

mod common;

use thorvg::{ColorSpace, EngineOption, Shape, SwCanvas, Thorvg};

fn main() {
    let _engine = Thorvg::init(0).expect("Failed to initialize ThorVG");

    let width = 800u32;
    let height = 600u32;
    let mut buffer = vec![0u32; (width * height) as usize];

    let mut canvas = SwCanvas::new(EngineOption::Default).expect("Failed to create canvas");
    canvas
        .set_target(&mut buffer, width, width, height, ColorSpace::ABGR8888)
        .unwrap();

    // ── White background ───────────────────────────────────────────
    let mut bg = Shape::new();
    bg.append_rect(0.0, 0.0, width as f32, height as f32, 0.0, 0.0, true)
        .unwrap();
    bg.set_fill_color(255, 255, 255, 255).unwrap();
    canvas.push(bg).unwrap();

    // ── Red rectangle ──────────────────────────────────────────────
    let mut rect = Shape::new();
    rect.append_rect(50.0, 50.0, 200.0, 150.0, 0.0, 0.0, true)
        .unwrap();
    rect.set_fill_color(255, 0, 0, 255).unwrap();
    canvas.push(rect).unwrap();

    // ── Rounded green rectangle ────────────────────────────────────
    let mut rounded = Shape::new();
    rounded
        .append_rect(300.0, 50.0, 200.0, 150.0, 20.0, 20.0, true)
        .unwrap();
    rounded.set_fill_color(0, 200, 0, 255).unwrap();
    canvas.push(rounded).unwrap();

    // ── Blue circle ────────────────────────────────────────────────
    let mut circle = Shape::new();
    circle
        .append_circle(150.0, 400.0, 80.0, 80.0, true)
        .unwrap();
    circle.set_fill_color(0, 0, 255, 255).unwrap();
    canvas.push(circle).unwrap();

    // ── Yellow ellipse ─────────────────────────────────────────────
    let mut ellipse = Shape::new();
    ellipse
        .append_circle(400.0, 400.0, 120.0, 60.0, true)
        .unwrap();
    ellipse.set_fill_color(255, 255, 0, 255).unwrap();
    canvas.push(ellipse).unwrap();

    // ── Orange triangle (custom path) ──────────────────────────────
    let mut triangle = Shape::new();
    triangle.move_to(650.0, 50.0).unwrap();
    triangle.line_to(750.0, 200.0).unwrap();
    triangle.line_to(550.0, 200.0).unwrap();
    triangle.close().unwrap();
    triangle.set_fill_color(255, 128, 0, 255).unwrap();
    canvas.push(triangle).unwrap();

    // ── Magenta cubic Bézier curve ─────────────────────────────────
    let mut curve = Shape::new();
    curve.move_to(550.0, 350.0).unwrap();
    curve
        .cubic_to(600.0, 250.0, 700.0, 550.0, 750.0, 350.0)
        .unwrap();
    curve.set_stroke_width(4.0).unwrap();
    curve.set_stroke_color(255, 0, 255, 255).unwrap();
    canvas.push(curve).unwrap();

    // ── Render & save ──────────────────────────────────────────────
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();

    common::save_png("shapes.png", &buffer, width, height);
}
