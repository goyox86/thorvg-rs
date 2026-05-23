//! Demonstrates stroke properties: width, color, cap, join, and dash patterns.
//!
//! Ported from `ThorVG`'s `testShape.cpp` "Stroking" test case.
//!
//! Run with: `cargo run --example stroke`
//! Output:   `stroke.png`

#![allow(clippy::cast_precision_loss)]

mod common;

use thorvg::{ColorSpace, EngineOption, Shape, StrokeCap, StrokeJoin, SwCanvas, Thorvg};

fn main() {
    let _engine = Thorvg::init(0).expect("Failed to initialize ThorVG");

    let width = 800u32;
    let height = 400u32;
    let mut buffer = vec![0u32; (width * height) as usize];

    let mut canvas = SwCanvas::new(EngineOption::Default).expect("Failed to create canvas");
    canvas
        .set_target(&mut buffer, width, width, height, ColorSpace::ABGR8888)
        .unwrap();

    // ── Dark background ────────────────────────────────────────────
    let mut bg = Shape::new();
    bg.append_rect(0.0, 0.0, width as f32, height as f32, 0.0, 0.0, true)
        .unwrap();
    bg.set_fill_color(30, 30, 30, 255).unwrap();
    canvas.push(bg).unwrap();

    // ── Butt cap, Miter join ───────────────────────────────────────
    let mut shape1 = Shape::new();
    shape1.move_to(50.0, 50.0).unwrap();
    shape1.line_to(200.0, 50.0).unwrap();
    shape1.line_to(200.0, 150.0).unwrap();
    shape1.set_stroke_width(8.0).unwrap();
    shape1.set_stroke_color(255, 80, 80, 255).unwrap();
    shape1.set_stroke_cap(StrokeCap::Butt).unwrap();
    shape1.set_stroke_join(StrokeJoin::Miter).unwrap();
    canvas.push(shape1).unwrap();

    // ── Round cap, Round join ──────────────────────────────────────
    let mut shape2 = Shape::new();
    shape2.move_to(300.0, 50.0).unwrap();
    shape2.line_to(450.0, 50.0).unwrap();
    shape2.line_to(450.0, 150.0).unwrap();
    shape2.set_stroke_width(8.0).unwrap();
    shape2.set_stroke_color(80, 255, 80, 255).unwrap();
    shape2.set_stroke_cap(StrokeCap::Round).unwrap();
    shape2.set_stroke_join(StrokeJoin::Round).unwrap();
    canvas.push(shape2).unwrap();

    // ── Square cap, Bevel join ─────────────────────────────────────
    let mut shape3 = Shape::new();
    shape3.move_to(550.0, 50.0).unwrap();
    shape3.line_to(700.0, 50.0).unwrap();
    shape3.line_to(700.0, 150.0).unwrap();
    shape3.set_stroke_width(8.0).unwrap();
    shape3.set_stroke_color(80, 80, 255, 255).unwrap();
    shape3.set_stroke_cap(StrokeCap::Square).unwrap();
    shape3.set_stroke_join(StrokeJoin::Bevel).unwrap();
    canvas.push(shape3).unwrap();

    // ── Dashed stroke ──────────────────────────────────────────────
    let mut dashed = Shape::new();
    dashed
        .append_rect(50.0, 250.0, 300.0, 100.0, 0.0, 0.0, true)
        .unwrap();
    dashed.set_stroke_width(4.0).unwrap();
    dashed.set_stroke_color(255, 255, 80, 255).unwrap();
    dashed
        .set_stroke_dash(&[15.0, 10.0, 5.0, 10.0], 0.0)
        .unwrap();
    canvas.push(dashed).unwrap();

    // ── Stroked circle with fill ───────────────────────────────────
    let mut stroked_circle = Shape::new();
    stroked_circle
        .append_circle(600.0, 300.0, 60.0, 60.0, true)
        .unwrap();
    stroked_circle.set_fill_color(100, 100, 255, 128).unwrap();
    stroked_circle.set_stroke_width(6.0).unwrap();
    stroked_circle.set_stroke_color(255, 255, 255, 255).unwrap();
    canvas.push(stroked_circle).unwrap();

    // ── Render & save ──────────────────────────────────────────────
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();

    common::save_png("stroke.png", &buffer, width, height);
}
