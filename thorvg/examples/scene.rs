//! Demonstrates scene composition: grouping shapes, applying transforms and effects.
//!
//! Ported from `ThorVG`'s `testScene.cpp` "Scene Effects" test case.
//!
//! Run with: `cargo run --example scene`
//! Output:   `scene.png`

#![allow(clippy::cast_precision_loss)]

mod common;

use thorvg::{ColorSpace, EngineOption, Paint, Thorvg};

fn main() {
    let engine = Thorvg::init(0).expect("Failed to initialize ThorVG");

    let width = 600u32;
    let height = 400u32;
    let mut buffer = vec![0u32; (width * height) as usize];

    let mut canvas = engine.sw_canvas(EngineOption::Default).expect("Failed to create canvas");
    unsafe { canvas.set_target(&mut buffer, width, width, height, ColorSpace::ABGR8888) }.unwrap();

    // ── Background ─────────────────────────────────────────────────
    let mut bg = engine.shape().unwrap();
    bg.append_rect(0.0, 0.0, width as f32, height as f32, 0.0, 0.0, true)
        .unwrap();
    bg.set_fill_color(240, 240, 240, 255).unwrap();
    canvas.push(bg).unwrap();

    // ── Scene with grouped shapes ──────────────────────────────────
    let mut scene = engine.scene().unwrap();

    // Green circle
    let mut circle = engine.shape().unwrap();
    circle
        .append_circle(150.0, 150.0, 80.0, 80.0, true)
        .unwrap();
    circle.set_fill_color(0, 200, 0, 255).unwrap();
    scene.push(circle).unwrap();

    // Semi-transparent blue rectangle overlapping the circle
    let mut rect = engine.shape().unwrap();
    rect.append_rect(100.0, 100.0, 150.0, 100.0, 10.0, 10.0, true)
        .unwrap();
    rect.set_fill_color(0, 0, 200, 180).unwrap();
    scene.push(rect).unwrap();

    // Yellow star-like shape using paths
    let mut star = engine.shape().unwrap();
    star.move_to(350.0, 80.0).unwrap();
    star.line_to(370.0, 140.0).unwrap();
    star.line_to(430.0, 140.0).unwrap();
    star.line_to(380.0, 180.0).unwrap();
    star.line_to(400.0, 240.0).unwrap();
    star.line_to(350.0, 200.0).unwrap();
    star.line_to(300.0, 240.0).unwrap();
    star.line_to(320.0, 180.0).unwrap();
    star.line_to(270.0, 140.0).unwrap();
    star.line_to(330.0, 140.0).unwrap();
    star.close().unwrap();
    star.set_fill_color(255, 220, 0, 255).unwrap();
    scene.push(star).unwrap();

    // Apply scene-level transform
    scene.translate(30.0, 40.0).unwrap();

    // Apply Gaussian blur to the entire scene
    scene.add_gaussian_blur(3.0, 0, 0, 80).unwrap();

    canvas.push(scene).unwrap();

    // ── Render & save ──────────────────────────────────────────────
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();

    common::save_png("scene.png", &buffer, width, height);
}
