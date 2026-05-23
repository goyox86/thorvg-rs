//! Demonstrates blending modes: overlapping shapes with different blend methods.
//!
//! Run with: `cargo run --example blending`
//! Output:   `blending.png`

#![allow(clippy::cast_precision_loss)]

mod common;

use thorvg::{BlendMethod, ColorSpace, EngineOption, Paint, Shape, SwCanvas, Thorvg};

fn main() {
    let _engine = Thorvg::init(0).unwrap();
    let (w, h) = (900u32, 500u32);
    let mut buffer = vec![0u32; (w * h) as usize];
    let mut canvas = SwCanvas::new(EngineOption::Default).unwrap();
    canvas
        .set_target(&mut buffer, w, w, h, ColorSpace::ABGR8888)
        .unwrap();

    // Background
    let mut bg = Shape::new();
    bg.append_rect(0.0, 0.0, w as f32, h as f32, 0.0, 0.0, true)
        .unwrap();
    bg.set_fill_color(240, 240, 240, 255).unwrap();
    canvas.push(bg).unwrap();

    let modes: &[(BlendMethod, &str)] = &[
        (BlendMethod::Normal, "Normal"),
        (BlendMethod::Multiply, "Multiply"),
        (BlendMethod::Screen, "Screen"),
        (BlendMethod::Overlay, "Overlay"),
        (BlendMethod::Darken, "Darken"),
        (BlendMethod::Lighten, "Lighten"),
        (BlendMethod::ColorDodge, "Dodge"),
        (BlendMethod::ColorBurn, "Burn"),
        (BlendMethod::Difference, "Diff"),
        (BlendMethod::Exclusion, "Exclusion"),
        (BlendMethod::Add, "Add"),
        (BlendMethod::SoftLight, "SoftLight"),
    ];

    let cols = 4;
    let cell_w = 200.0f32;
    let cell_h = 140.0f32;
    let margin = 15.0f32;

    for (i, (mode, _name)) in modes.iter().enumerate() {
        let col = (i % cols) as f32;
        let row = (i / cols) as f32;
        let x = margin + col * (cell_w + margin);
        let y = margin + row * (cell_h + margin);

        // Red circle (bottom layer)
        let mut circle1 = Shape::new();
        circle1
            .append_circle(x + 70.0, y + 60.0, 45.0, 45.0, true)
            .unwrap();
        circle1.set_fill_color(220, 40, 40, 255).unwrap();
        canvas.push(circle1).unwrap();

        // Blue circle (top layer with blend)
        let mut circle2 = Shape::new();
        circle2
            .append_circle(x + 110.0, y + 60.0, 45.0, 45.0, true)
            .unwrap();
        circle2.set_fill_color(40, 40, 220, 255).unwrap();
        circle2.set_blend(*mode).unwrap();
        canvas.push(circle2).unwrap();
    }

    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
    common::save_png("blending.png", &buffer, w, h);
    println!(
        "Blend modes shown: {}",
        modes
            .iter()
            .map(|(_, n)| *n)
            .collect::<alloc::vec::Vec<_>>()
            .join(", ")
    );
}

extern crate alloc;
