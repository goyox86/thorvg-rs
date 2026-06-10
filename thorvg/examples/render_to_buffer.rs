//! Demonstrates rendering to a raw pixel buffer — the typical embedded/`no_std` use case.
//!
//! This example renders shapes into a `u32` buffer and writes the result to a PNG file.
//!
//! Run with: `cargo run --example render_to_buffer`
//! Output:   `render_to_buffer.png`

#![allow(clippy::cast_precision_loss)]

mod common;

use thorvg::{ColorSpace, ColorStop, EngineOption, Paint, Thorvg};

fn main() {
    let engine = Thorvg::init(0).expect("Failed to initialize ThorVG");

    let width = 400u32;
    let height = 300u32;
    let mut buffer = vec![0u32; (width * height) as usize];

    let mut canvas = engine.sw_canvas(EngineOption::Default).expect("Failed to create canvas");
    unsafe { canvas.set_target(&mut buffer, width, width, height, ColorSpace::ABGR8888) }.unwrap();

    // ── Background: white rectangle ────────────────────────────────
    let mut bg = engine.shape().unwrap();
    bg.append_rect(0.0, 0.0, width as f32, height as f32, 0.0, 0.0, true)
        .unwrap();
    bg.set_fill_color(255, 255, 255, 255).unwrap();
    canvas.add(bg).unwrap();

    // ── Gradient-filled rounded rect ───────────────────────────────
    let mut grad = engine.linear_gradient().unwrap();
    grad.set_bounds(50.0, 50.0, 350.0, 250.0).unwrap();
    grad.set_color_stops(&[
        ColorStop {
            offset: 0.0,
            r: 64,
            g: 0,
            b: 128,
            a: 255,
        },
        ColorStop {
            offset: 0.5,
            r: 0,
            g: 128,
            b: 255,
            a: 255,
        },
        ColorStop {
            offset: 1.0,
            r: 0,
            g: 255,
            b: 128,
            a: 255,
        },
    ])
    .unwrap();

    let mut rect = engine.shape().unwrap();
    rect.append_rect(50.0, 50.0, 300.0, 200.0, 25.0, 25.0, true)
        .unwrap();
    rect.set_linear_gradient(grad).unwrap();
    canvas.add(rect).unwrap();

    // ── Semi-transparent white circle on top ───────────────────────
    let mut circle = engine.shape().unwrap();
    circle
        .append_circle(200.0, 150.0, 50.0, 50.0, true)
        .unwrap();
    circle.set_fill_color(255, 255, 255, 200).unwrap();
    circle.set_opacity(200).unwrap();
    canvas.add(circle).unwrap();

    // ── Render & save ──────────────────────────────────────────────
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();

    common::save_png("render_to_buffer.png", &buffer, width, height);
}
