//! Demonstrates scene-level effects: drop shadow, fill, tint, and tritone.
//!
//! Run with: `cargo run --example scene_effects`
//! Output:   `scene_effects.png`

#![allow(clippy::cast_precision_loss)]

mod common;

use thorvg::{
    BlurBorder, BlurDirection, ColorSpace, DropShadow, EngineOption, Paint, Rgb, Rgba, Scene,
    Thorvg, Tint, Tritone,
};

fn make_shape_group(engine: &Thorvg, x: f32, y: f32) -> Scene<'_> {
    let mut scene = engine.scene().unwrap();

    let mut circle = engine.shape().unwrap();
    circle
        .append_circle(x + 60.0, y + 60.0, 40.0, 40.0, true)
        .unwrap();
    circle.set_fill_color(0, 180, 80, 255).unwrap();
    scene.push(circle).unwrap();

    let mut rect = engine.shape().unwrap();
    rect.append_rect(x + 30.0, y + 30.0, 80.0, 60.0, 8.0, 8.0, true)
        .unwrap();
    rect.set_fill_color(80, 80, 220, 200).unwrap();
    scene.push(rect).unwrap();

    scene
}

fn main() {
    let engine = Thorvg::init(0).unwrap();
    let (w, h) = (800u32, 500u32);
    let mut buffer = vec![0u32; (w * h) as usize];
    let mut canvas = engine.sw_canvas(EngineOption::Default).unwrap();
    unsafe { canvas.set_target(&mut buffer, w, w, h, ColorSpace::ABGR8888) }.unwrap();

    // Background
    let mut bg = engine.shape().unwrap();
    bg.append_rect(0.0, 0.0, w as f32, h as f32, 0.0, 0.0, true)
        .unwrap();
    bg.set_fill_color(245, 245, 245, 255).unwrap();
    canvas.push(bg).unwrap();

    // ── No effect (reference) ──────────────────────────────────────
    let scene1 = make_shape_group(&engine, 30.0, 30.0);
    canvas.push(scene1).unwrap();

    // ── Gaussian blur ──────────────────────────────────────────────
    let mut scene2 = make_shape_group(&engine, 220.0, 30.0);
    scene2
        .add_gaussian_blur_effect(5.0, BlurDirection::Both, BlurBorder::Duplicate, 80)
        .unwrap();
    canvas.push(scene2).unwrap();

    // ── Drop shadow ────────────────────────────────────────────────
    let mut scene3 = make_shape_group(&engine, 410.0, 30.0);
    scene3
        .add_drop_shadow_effect(DropShadow {
            color: Rgba::new(0, 0, 0, 150),
            angle: 135.0,
            distance: 8.0,
            sigma: 4.0,
            quality: 80,
        })
        .unwrap();
    canvas.push(scene3).unwrap();

    // ── Fill effect (recolor) ──────────────────────────────────────
    let mut scene4 = make_shape_group(&engine, 600.0, 30.0);
    scene4.add_fill_effect(Rgba::new(255, 60, 60, 200)).unwrap();
    canvas.push(scene4).unwrap();

    // ── Tint effect ────────────────────────────────────────────────
    let mut scene5 = make_shape_group(&engine, 30.0, 250.0);
    // Builder form to demonstrate the chainable API.
    scene5
        .add_tint_effect(
            Tint::new()
                .black(Rgb::new(20, 0, 40))
                .white(Rgb::new(255, 200, 180))
                .intensity(80.0),
        )
        .unwrap();
    canvas.push(scene5).unwrap();

    // ── Tritone effect ─────────────────────────────────────────────
    let mut scene6 = make_shape_group(&engine, 220.0, 250.0);
    scene6
        .add_tritone_effect(Tritone {
            shadow: Rgb::new(10, 10, 40),
            midtone: Rgb::new(200, 100, 50),
            highlight: Rgb::new(255, 240, 200),
            blend: 180,
        })
        .unwrap();
    canvas.push(scene6).unwrap();

    // ── Blur + drop shadow (stacked) ───────────────────────────────
    let mut scene7 = make_shape_group(&engine, 410.0, 250.0);
    scene7
        .add_gaussian_blur_effect(2.0, BlurDirection::Both, BlurBorder::Duplicate, 60)
        .unwrap();
    // Same effect, expressed via the chainable builder — fields
    // not mentioned inherit `DropShadow::new()`'s sensible defaults.
    scene7
        .add_drop_shadow_effect(
            DropShadow::new()
                .color(Rgba::new(0, 0, 0, 100))
                .angle(45.0)
                .distance(12.0)
                .sigma(6.0)
                .quality(80),
        )
        .unwrap();
    canvas.push(scene7).unwrap();

    // ── Scene with transform + blur ────────────────────────────────
    let mut scene8 = make_shape_group(&engine, 600.0, 250.0);
    scene8.scale(0.8).unwrap();
    scene8.translate(720.0, 310.0).unwrap();
    scene8
        .add_gaussian_blur_effect(3.0, BlurDirection::Horizontal, BlurBorder::Duplicate, 70)
        .unwrap();
    canvas.push(scene8).unwrap();

    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
    common::save_png("scene_effects.png", &buffer, w, h);
}
