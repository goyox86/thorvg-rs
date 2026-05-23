//! Demonstrates scene-level effects: drop shadow, fill, tint, and tritone.
//!
//! Run with: `cargo run --example scene_effects`
//! Output:   `scene_effects.png`

#![allow(clippy::cast_precision_loss)]

mod common;

use thorvg::{ColorSpace, EngineOption, Paint, Scene, Shape, SwCanvas, Thorvg};

fn make_shape_group(x: f32, y: f32) -> Scene {
    let mut scene = Scene::new();

    let mut circle = Shape::new();
    circle
        .append_circle(x + 60.0, y + 60.0, 40.0, 40.0, true)
        .unwrap();
    circle.set_fill_color(0, 180, 80, 255).unwrap();
    scene.push(circle).unwrap();

    let mut rect = Shape::new();
    rect.append_rect(x + 30.0, y + 30.0, 80.0, 60.0, 8.0, 8.0, true)
        .unwrap();
    rect.set_fill_color(80, 80, 220, 200).unwrap();
    scene.push(rect).unwrap();

    scene
}

fn main() {
    let _engine = Thorvg::init(0).unwrap();
    let (w, h) = (800u32, 500u32);
    let mut buffer = vec![0u32; (w * h) as usize];
    let mut canvas = SwCanvas::new(EngineOption::Default).unwrap();
    unsafe { canvas.set_target(&mut buffer, w, w, h, ColorSpace::ABGR8888) }.unwrap();

    // Background
    let mut bg = Shape::new();
    bg.append_rect(0.0, 0.0, w as f32, h as f32, 0.0, 0.0, true)
        .unwrap();
    bg.set_fill_color(245, 245, 245, 255).unwrap();
    canvas.push(bg).unwrap();

    // ── No effect (reference) ──────────────────────────────────────
    let scene1 = make_shape_group(30.0, 30.0);
    canvas.push(scene1).unwrap();

    // ── Gaussian blur ──────────────────────────────────────────────
    let mut scene2 = make_shape_group(220.0, 30.0);
    scene2.add_gaussian_blur(5.0, 0, 0, 80).unwrap();
    canvas.push(scene2).unwrap();

    // ── Drop shadow ────────────────────────────────────────────────
    let mut scene3 = make_shape_group(410.0, 30.0);
    scene3
        .add_drop_shadow(0, 0, 0, 150, 135.0, 8.0, 4.0, 80)
        .unwrap();
    canvas.push(scene3).unwrap();

    // ── Fill effect (recolor) ──────────────────────────────────────
    let mut scene4 = make_shape_group(600.0, 30.0);
    scene4.add_fill_effect(255, 60, 60, 200).unwrap();
    canvas.push(scene4).unwrap();

    // ── Tint effect ────────────────────────────────────────────────
    let mut scene5 = make_shape_group(30.0, 250.0);
    scene5
        .add_tint_effect(20, 0, 40, 255, 200, 180, 80.0)
        .unwrap();
    canvas.push(scene5).unwrap();

    // ── Tritone effect ─────────────────────────────────────────────
    let mut scene6 = make_shape_group(220.0, 250.0);
    scene6
        .add_tritone_effect(
            10, 10, 40, // shadow
            200, 100, 50, // midtone
            255, 240, 200, // highlight
            180, // blend
        )
        .unwrap();
    canvas.push(scene6).unwrap();

    // ── Blur + drop shadow (stacked) ───────────────────────────────
    let mut scene7 = make_shape_group(410.0, 250.0);
    scene7.add_gaussian_blur(2.0, 0, 0, 60).unwrap();
    scene7
        .add_drop_shadow(0, 0, 0, 100, 45.0, 12.0, 6.0, 80)
        .unwrap();
    canvas.push(scene7).unwrap();

    // ── Scene with transform + blur ────────────────────────────────
    let mut scene8 = make_shape_group(600.0, 250.0);
    scene8.scale(0.8).unwrap();
    scene8.translate(720.0, 310.0).unwrap();
    scene8.add_gaussian_blur(3.0, 1, 0, 70).unwrap();
    canvas.push(scene8).unwrap();

    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
    common::save_png("scene_effects.png", &buffer, w, h);
}
