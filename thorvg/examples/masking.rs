//! Demonstrates masking: alpha and luma mask methods.
//!
//! Run with: `cargo run --example masking`
//! Output:   `masking.png`

#![allow(clippy::cast_precision_loss)]

mod common;

use thorvg::{
    ColorSpace, ColorStop, EngineOption, LinearGradient, MaskMethod, Paint, Shape, SwCanvas, Thorvg,
};

fn main() {
    let _engine = Thorvg::init(0).unwrap();
    let (w, h) = (700u32, 350u32);
    let mut buffer = vec![0u32; (w * h) as usize];
    let mut canvas = SwCanvas::new(EngineOption::Default).unwrap();
    unsafe { canvas.set_target(&mut buffer, w, w, h, ColorSpace::ABGR8888) }.unwrap();

    // Checkerboard background
    for row in 0..(h / 15) {
        for col in 0..(w / 15) {
            let mut sq = Shape::new();
            sq.append_rect(
                col as f32 * 15.0,
                row as f32 * 15.0,
                15.0,
                15.0,
                0.0,
                0.0,
                true,
            )
            .unwrap();
            let gray = if (row + col) % 2 == 0 { 180 } else { 210 };
            sq.set_fill_color(gray, gray, gray, 255).unwrap();
            canvas.push(sq).unwrap();
        }
    }

    // ── Alpha mask: gradient circle masks a rectangle ───────────────
    let mut rect1 = Shape::new();
    rect1
        .append_rect(30.0, 50.0, 200.0, 200.0, 0.0, 0.0, true)
        .unwrap();
    rect1.set_fill_color(50, 50, 220, 255).unwrap();

    // Mask target: circle with gradient alpha
    let mut grad = LinearGradient::new();
    grad.set_bounds(30.0, 50.0, 230.0, 250.0).unwrap();
    grad.set_color_stops(&[
        ColorStop {
            offset: 0.0,
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        ColorStop {
            offset: 1.0,
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
    ])
    .unwrap();

    let mut mask1 = Shape::new();
    mask1.append_circle(130.0, 150.0, 90.0, 90.0, true).unwrap();
    mask1.set_linear_gradient(grad).unwrap();

    rect1.set_mask(&mask1, MaskMethod::Alpha).unwrap();
    canvas.push(rect1).unwrap();

    // ── InvAlpha mask ──────────────────────────────────────────────
    let mut rect2 = Shape::new();
    rect2
        .append_rect(260.0, 50.0, 200.0, 200.0, 0.0, 0.0, true)
        .unwrap();
    rect2.set_fill_color(220, 50, 50, 255).unwrap();

    let mut mask2 = Shape::new();
    mask2.append_circle(360.0, 150.0, 60.0, 60.0, true).unwrap();
    mask2.set_fill_color(255, 255, 255, 255).unwrap();

    rect2.set_mask(&mask2, MaskMethod::InvAlpha).unwrap();
    canvas.push(rect2).unwrap();

    // ── Luma mask ──────────────────────────────────────────────────
    let mut rect3 = Shape::new();
    rect3
        .append_rect(490.0, 50.0, 180.0, 200.0, 15.0, 15.0, true)
        .unwrap();
    rect3.set_fill_color(50, 200, 50, 255).unwrap();

    let mut grad3 = LinearGradient::new();
    grad3.set_bounds(490.0, 50.0, 670.0, 250.0).unwrap();
    grad3
        .set_color_stops(&[
            ColorStop {
                offset: 0.0,
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            ColorStop {
                offset: 1.0,
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
        ])
        .unwrap();

    let mut mask3 = Shape::new();
    mask3
        .append_rect(490.0, 50.0, 180.0, 200.0, 0.0, 0.0, true)
        .unwrap();
    mask3.set_linear_gradient(grad3).unwrap();

    rect3.set_mask(&mask3, MaskMethod::Luma).unwrap();
    canvas.push(rect3).unwrap();

    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
    common::save_png("masking.png", &buffer, w, h);
}
