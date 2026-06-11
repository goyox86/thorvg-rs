//! Demonstrates gradient fills: linear and radial gradients with multiple color stops.
//!
//! Ported from `ThorVG`'s `testFill.cpp` examples.
//!
//! Run with: `cargo run --example gradient`
//! Output:   `gradient.png`

#![allow(clippy::cast_precision_loss, clippy::too_many_lines)]

mod common;

use thorvg::{Circle, ColorSpace, ColorStop, EngineOption, FillSpread, Paint, Rect, Rgba, Thorvg};

fn main() {
    let engine = Thorvg::init(0).expect("Failed to initialize ThorVG");

    let width = 800u32;
    let height = 400u32;
    let mut buffer = vec![0u32; (width * height) as usize];

    let mut canvas = engine
        .sw_canvas(EngineOption::DEFAULT)
        .expect("Failed to create canvas");
    unsafe { canvas.set_target(&mut buffer, width, width, height, ColorSpace::ABGR8888) }.unwrap();

    // ── Dark background ────────────────────────────────────────────
    let mut bg = engine.shape().unwrap();
    bg.append_rect(Rect::new(0.0, 0.0, width as f32, height as f32))
    .unwrap();
    bg.set_fill_color(Rgba::new(20, 20, 20, 255)).unwrap();
    canvas.add(bg).unwrap();

    // ── Linear gradient: red → blue ────────────────────────────────
    let mut linear_grad = engine.linear_gradient().unwrap();
    linear_grad.set_bounds(50.0, 50.0, 350.0, 200.0).unwrap();
    linear_grad
        .set_color_stops(&[
            ColorStop {
                offset: 0.0,
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
            ColorStop {
                offset: 1.0,
                r: 0,
                g: 0,
                b: 255,
                a: 255,
            },
        ])
        .unwrap();

    let mut rect1 = engine.shape().unwrap();
    rect1
        .append_rect(Rect::new(50.0, 50.0, 300.0, 150.0))
        .unwrap();
    rect1.set_linear_gradient(linear_grad).unwrap();
    canvas.add(rect1).unwrap();

    // ── Linear gradient with 4 color stops (rainbow) ───────────────
    let mut rainbow_grad = engine.linear_gradient().unwrap();
    rainbow_grad.set_bounds(50.0, 250.0, 350.0, 250.0).unwrap();
    rainbow_grad
        .set_color_stops(&[
            ColorStop {
                offset: 0.0,
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
            ColorStop {
                offset: 0.33,
                r: 255,
                g: 255,
                b: 0,
                a: 255,
            },
            ColorStop {
                offset: 0.66,
                r: 0,
                g: 255,
                b: 0,
                a: 255,
            },
            ColorStop {
                offset: 1.0,
                r: 0,
                g: 0,
                b: 255,
                a: 255,
            },
        ])
        .unwrap();
    rainbow_grad.set_spread(FillSpread::Pad).unwrap();

    let mut rect2 = engine.shape().unwrap();
    rect2
        .append_rect(Rect::new(50.0, 250.0, 300.0, 100.0))
        .unwrap();
    rect2.set_linear_gradient(rainbow_grad).unwrap();
    canvas.add(rect2).unwrap();

    // ── Radial gradient on a circle ────────────────────────────────
    let mut radial_grad = engine.radial_gradient().unwrap();
    radial_grad
        .set_radial(550.0, 130.0, 100.0, 550.0, 130.0, 0.0)
        .unwrap();
    radial_grad
        .set_color_stops(&[
            ColorStop {
                offset: 0.0,
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            ColorStop {
                offset: 0.5,
                r: 255,
                g: 200,
                b: 0,
                a: 255,
            },
            ColorStop {
                offset: 1.0,
                r: 200,
                g: 0,
                b: 0,
                a: 255,
            },
        ])
        .unwrap();

    let mut circle = engine.shape().unwrap();
    circle
        .append_circle(Circle::new(550.0, 130.0, 100.0))
        .unwrap();
    circle.set_radial_gradient(radial_grad).unwrap();
    canvas.add(circle).unwrap();

    // ── Radial gradient with focal offset ──────────────────────────
    let mut focal_grad = engine.radial_gradient().unwrap();
    focal_grad
        .set_radial(600.0, 310.0, 80.0, 560.0, 280.0, 10.0)
        .unwrap();
    focal_grad
        .set_color_stops(&[
            ColorStop {
                offset: 0.0,
                r: 0,
                g: 255,
                b: 0,
                a: 255,
            },
            ColorStop {
                offset: 1.0,
                r: 0,
                g: 0,
                b: 128,
                a: 255,
            },
        ])
        .unwrap();

    let mut rect3 = engine.shape().unwrap();
    rect3
        .append_rect(Rect::new(500.0, 250.0, 200.0, 120.0).corner_radius(15.0))
        .unwrap();
    rect3.set_radial_gradient(focal_grad).unwrap();
    rect3.set_opacity(200).unwrap();
    canvas.add(rect3).unwrap();

    // ── Render & save ──────────────────────────────────────────────
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();

    common::save_png("gradient.png", &buffer, width, height);
}
