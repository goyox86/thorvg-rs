//! Demonstrates shape clipping: restricting a shape's rendering to a clip path.
//!
//! Run with: `cargo run --example clipping`
//! Output:   `clipping.png`

#![allow(clippy::cast_precision_loss)]

mod common;

use thorvg::{ColorSpace, ColorStop, EngineOption, Paint, Rgba, Circle, Rect, Thorvg};

fn main() {
    let engine = Thorvg::init(0).unwrap();
    let (w, h) = (600u32, 400u32);
    let mut buffer = vec![0u32; (w * h) as usize];
    let mut canvas = engine.sw_canvas(EngineOption::Default).unwrap();
    unsafe { canvas.set_target(&mut buffer, w, w, h, ColorSpace::ABGR8888) }.unwrap();

    // Background
    let mut bg = engine.shape().unwrap();
    bg.append_rect(Rect { x: 0.0, y: 0.0, width: w as f32, height: h as f32, rx: 0.0, ry: 0.0, cw: true })
        .unwrap();
    bg.set_fill_color(Rgba::new(30, 30, 50, 255)).unwrap();
    canvas.add(bg).unwrap();

    // ── Left: gradient rectangle clipped by a circle ───────────────
    let mut grad = engine.linear_gradient().unwrap();
    grad.set_bounds(50.0, 50.0, 250.0, 300.0).unwrap();
    grad.set_color_stops(&[
        ColorStop {
            offset: 0.0,
            r: 255,
            g: 100,
            b: 0,
            a: 255,
        },
        ColorStop {
            offset: 1.0,
            r: 0,
            g: 100,
            b: 255,
            a: 255,
        },
    ])
    .unwrap();

    let mut rect = engine.shape().unwrap();
    rect.append_rect(Rect { x: 50.0, y: 50.0, width: 200.0, height: 300.0, rx: 0.0, ry: 0.0, cw: true })
        .unwrap();
    rect.set_linear_gradient(grad).unwrap();

    // Circle clip
    let mut clip1 = engine.shape().unwrap();
    clip1.append_circle(Circle { cx: 150.0, cy: 200.0, rx: 90.0, ry: 90.0, cw: true }).unwrap();
    rect.set_clip(clip1).unwrap();

    canvas.add(rect).unwrap();

    // ── Right: star shape clipped by a rounded rectangle ───────────
    let mut star = engine.shape().unwrap();
    let cx = 440.0f32;
    let cy = 200.0f32;
    let spikes = 6;
    let outer = 120.0f32;
    let inner = 55.0f32;

    for i in 0..(spikes * 2) {
        let angle = core::f32::consts::PI * i as f32 / spikes as f32 - core::f32::consts::FRAC_PI_2;
        let r = if i % 2 == 0 { outer } else { inner };
        let px = cx + angle.cos() * r;
        let py = cy + angle.sin() * r;
        if i == 0 {
            star.move_to(px, py).unwrap();
        } else {
            star.line_to(px, py).unwrap();
        }
    }
    star.close().unwrap();
    star.set_fill_color(Rgba::new(255, 220, 0, 255)).unwrap();

    // Rounded rectangle clip
    let mut clip2 = engine.shape().unwrap();
    clip2.append_rect(Rect { x: 360.0, y: 100.0, width: 160.0, height: 200.0, rx: 20.0, ry: 20.0, cw: true })
        .unwrap();
    star.set_clip(clip2).unwrap();

    canvas.add(star).unwrap();

    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
    common::save_png("clipping.png", &buffer, w, h);
}
