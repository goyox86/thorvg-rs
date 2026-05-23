#![allow(clippy::cast_precision_loss)]

extern crate alloc;
extern crate std;

use crate::*;
use alloc::vec;
use std::sync::OnceLock;

/// Shared engine guard — initialized once, kept alive for all tests.
fn init_engine() -> &'static Thorvg {
    static ENGINE: OnceLock<Thorvg> = OnceLock::new();
    ENGINE.get_or_init(|| Thorvg::init(0).expect("Failed to init ThorVG"))
}

// ── Engine & Version ───────────────────────────────────────────────

#[test]
fn test_init_and_version() {
    let _guard = init_engine();
    let (major, _minor, _micro, version_str) = Thorvg::version().expect("Failed to get version");
    assert!(major >= 1);
    assert!(!version_str.is_empty());
}

// ── Canvas Lifecycle ───────────────────────────────────────────────

#[test]
fn test_canvas_create_destroy() {
    let _guard = init_engine();
    // Canvas should be created and dropped without issues
    let canvas = SwCanvas::new(EngineOption::Default);
    assert!(canvas.is_ok());
    // Implicit drop here
}

#[test]
fn test_canvas_draw_shape() {
    let _guard = init_engine();
    let mut canvas = SwCanvas::new(EngineOption::Default).unwrap();
    let (width, height) = (100u32, 100u32);
    let mut buffer = vec![0u32; (width * height) as usize];

    unsafe { canvas.set_target(&mut buffer, width, width, height, ColorSpace::ABGR8888) }.unwrap();

    let mut shape = Shape::new();
    shape
        .append_rect(10.0, 10.0, 50.0, 50.0, 0.0, 0.0, true)
        .unwrap();
    shape.set_fill_color(255, 0, 0, 255).unwrap();

    canvas.push(shape).unwrap(); // ownership transferred
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();

    assert!(
        buffer.iter().any(|&px| px != 0),
        "Expected non-empty render output"
    );
}

#[test]
fn test_canvas_clear_all() {
    let _guard = init_engine();
    let mut canvas = SwCanvas::new(EngineOption::Default).unwrap();
    let mut buffer = vec![0u32; 100 * 100];
    unsafe { canvas.set_target(&mut buffer, 100, 100, 100, ColorSpace::ABGR8888) }.unwrap();

    // Push multiple shapes
    for _ in 0..5 {
        let mut s = Shape::new();
        s.append_rect(0.0, 0.0, 10.0, 10.0, 0.0, 0.0, true).unwrap();
        s.set_fill_color(255, 0, 0, 255).unwrap();
        canvas.push(s).unwrap();
    }

    // Clear all — should not double-free anything
    canvas.clear().unwrap();

    // Canvas should still be usable after clearing
    let mut s = Shape::new();
    s.append_rect(0.0, 0.0, 50.0, 50.0, 0.0, 0.0, true).unwrap();
    s.set_fill_color(0, 255, 0, 255).unwrap();
    canvas.push(s).unwrap();
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
}

// ── Shape Ownership ────────────────────────────────────────────────

#[test]
fn test_shape_ownership_transfer_to_canvas() {
    let _guard = init_engine();
    let mut canvas = SwCanvas::new(EngineOption::Default).unwrap();
    let mut buffer = vec![0u32; 100 * 100];
    unsafe { canvas.set_target(&mut buffer, 100, 100, 100, ColorSpace::ABGR8888) }.unwrap();

    let mut shape = Shape::new();
    shape
        .append_rect(0.0, 0.0, 50.0, 50.0, 0.0, 0.0, true)
        .unwrap();
    shape.set_fill_color(255, 0, 0, 255).unwrap();

    // Transfer ownership — shape should NOT be freed by Rust
    canvas.push(shape).unwrap();
    // shape is consumed, canvas owns it, drop of canvas frees it
}

#[test]
fn test_shape_ownership_transfer_to_scene() {
    let _guard = init_engine();
    let mut scene = Scene::new();

    let mut s1 = Shape::new();
    s1.append_rect(0.0, 0.0, 10.0, 10.0, 0.0, 0.0, true)
        .unwrap();
    s1.set_fill_color(255, 0, 0, 255).unwrap();

    let mut s2 = Shape::new();
    s2.append_circle(50.0, 50.0, 20.0, 20.0, true).unwrap();
    s2.set_fill_color(0, 0, 255, 255).unwrap();

    scene.push(s1).unwrap();
    scene.push(s2).unwrap();

    // Scene dropped here — must free both shapes without double-free
}

#[test]
fn test_shape_not_transferred_is_freed() {
    let _guard = init_engine();
    // Shape created but never pushed to canvas/scene — Rust must free it
    let mut shape = Shape::new();
    shape
        .append_rect(0.0, 0.0, 100.0, 100.0, 0.0, 0.0, true)
        .unwrap();
    shape.set_fill_color(255, 255, 0, 255).unwrap();
    shape.set_stroke_width(3.0).unwrap();
    shape.set_stroke_color(0, 0, 0, 255).unwrap();
    // Dropped here — must be freed by Paint::rel
}

// ── Gradient Ownership ─────────────────────────────────────────────

#[test]
fn test_gradient_ownership_transfer_to_shape() {
    let _guard = init_engine();

    let mut grad = LinearGradient::new();
    grad.set_bounds(0.0, 0.0, 100.0, 100.0).unwrap();
    grad.set_color_stops(&[
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

    let mut shape = Shape::new();
    shape
        .append_rect(0.0, 0.0, 100.0, 100.0, 0.0, 0.0, true)
        .unwrap();
    // Gradient ownership transferred to shape
    shape.set_linear_gradient(grad).unwrap();
    // Shape dropped here — must free gradient too
}

#[test]
fn test_gradient_not_transferred_is_freed() {
    let _guard = init_engine();
    // Gradient created but never given to a shape
    let mut grad = RadialGradient::new();
    grad.set_radial(50.0, 50.0, 30.0, 50.0, 50.0, 0.0).unwrap();
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
            a: 255,
        },
    ])
    .unwrap();
    // Dropped here — must be freed by gradient_del
}

#[test]
fn test_gradient_duplicate() {
    let _guard = init_engine();

    let mut grad = LinearGradient::new();
    grad.set_bounds(0.0, 0.0, 200.0, 200.0).unwrap();
    grad.set_color_stops(&[
        ColorStop {
            offset: 0.0,
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        },
        ColorStop {
            offset: 0.5,
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

    let dup = grad.duplicate();
    assert!(dup.is_some());

    let dup = dup.unwrap();
    let (x1, y1, x2, y2) = dup.bounds().unwrap();
    assert!((x1 - 0.0).abs() < f32::EPSILON);
    assert!((y1 - 0.0).abs() < f32::EPSILON);
    assert!((x2 - 200.0).abs() < f32::EPSILON);
    assert!((y2 - 200.0).abs() < f32::EPSILON);

    // Both original and duplicate dropped here — must not double-free
}

// ── Scene Lifecycle ────────────────────────────────────────────────

#[test]
fn test_scene_nested_drop() {
    let _guard = init_engine();

    let mut canvas = SwCanvas::new(EngineOption::Default).unwrap();
    let mut buffer = vec![0u32; 200 * 200];
    unsafe { canvas.set_target(&mut buffer, 200, 200, 200, ColorSpace::ABGR8888) }.unwrap();

    // Scene containing shapes, pushed to canvas
    let mut scene = Scene::new();
    for i in 0..10 {
        let mut s = Shape::new();
        s.append_circle(i as f32 * 20.0, 50.0, 10.0, 10.0, true)
            .unwrap();
        s.set_fill_color(255, 0, 0, 255).unwrap();
        scene.push(s).unwrap();
    }
    canvas.push(scene).unwrap();

    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
    // Canvas dropped → Scene dropped → 10 Shapes dropped, no double-free
}

#[test]
fn test_scene_clear_and_reuse() {
    let _guard = init_engine();

    let mut scene = Scene::new();

    // Add shapes
    for _ in 0..5 {
        let mut s = Shape::new();
        s.append_rect(0.0, 0.0, 10.0, 10.0, 0.0, 0.0, true).unwrap();
        scene.push(s).unwrap();
    }

    // Clear all shapes
    scene.clear().unwrap();

    // Scene should still be usable
    let mut s = Shape::new();
    s.append_rect(0.0, 0.0, 50.0, 50.0, 0.0, 0.0, true).unwrap();
    scene.push(s).unwrap();
}

// ── Paint Properties ───────────────────────────────────────────────

#[test]
fn test_shape_fill_color_roundtrip() {
    let _guard = init_engine();
    let mut shape = Shape::new();
    shape.set_fill_color(100, 150, 200, 255).unwrap();
    let (r, g, b, a) = shape.fill_color().unwrap();
    assert_eq!((r, g, b, a), (100, 150, 200, 255));
}

#[test]
fn test_shape_stroke_roundtrip() {
    let _guard = init_engine();
    let mut shape = Shape::new();
    shape.append_circle(50.0, 50.0, 30.0, 30.0, true).unwrap();
    shape.set_stroke_width(3.0).unwrap();
    shape.set_stroke_color(0, 255, 0, 255).unwrap();
    shape.set_stroke_cap(StrokeCap::Round).unwrap();
    shape.set_stroke_join(StrokeJoin::Bevel).unwrap();
    shape.set_stroke_miterlimit(8.0).unwrap();

    assert!((shape.stroke_width().unwrap() - 3.0).abs() < f32::EPSILON);
    let (r, g, b, a) = shape.stroke_color().unwrap();
    assert_eq!((r, g, b, a), (0, 255, 0, 255));
    assert_eq!(shape.stroke_cap().unwrap(), StrokeCap::Round);
    assert_eq!(shape.stroke_join().unwrap(), StrokeJoin::Bevel);
    assert!((shape.stroke_miterlimit().unwrap() - 8.0).abs() < f32::EPSILON);
}

#[test]
fn test_shape_fill_rule_roundtrip() {
    let _guard = init_engine();
    let mut shape = Shape::new();
    assert_eq!(shape.fill_rule().unwrap(), FillRule::NonZero);
    shape.set_fill_rule(FillRule::EvenOdd).unwrap();
    assert_eq!(shape.fill_rule().unwrap(), FillRule::EvenOdd);
}

#[test]
fn test_paint_opacity_roundtrip() {
    let _guard = init_engine();
    let mut shape = Shape::new();
    shape.set_opacity(128).unwrap();
    assert_eq!(shape.opacity().unwrap(), 128);
}

#[test]
fn test_paint_visibility_roundtrip() {
    let _guard = init_engine();
    let mut shape = Shape::new();
    assert!(shape.visible());
    shape.set_visible(false).unwrap();
    assert!(!shape.visible());
    shape.set_visible(true).unwrap();
    assert!(shape.visible());
}

#[test]
fn test_paint_transform_roundtrip() {
    let _guard = init_engine();
    let mut shape = Shape::new();
    let m = Matrix {
        e11: 2.0,
        e12: 0.5,
        e13: 10.0,
        e21: 0.0,
        e22: 3.0,
        e23: 20.0,
        e31: 0.0,
        e32: 0.0,
        e33: 1.0,
    };
    shape.set_transform(&m).unwrap();
    let got = shape.transform().unwrap();
    assert!((got.e11 - 2.0).abs() < f32::EPSILON);
    assert!((got.e12 - 0.5).abs() < f32::EPSILON);
    assert!((got.e13 - 10.0).abs() < f32::EPSILON);
    assert!((got.e22 - 3.0).abs() < f32::EPSILON);
    assert!((got.e23 - 20.0).abs() < f32::EPSILON);
}

#[test]
fn test_paint_id_roundtrip() {
    let _guard = init_engine();
    let mut shape = Shape::new();
    shape.set_id(42).unwrap();
    assert_eq!(shape.id(), 42);
}

#[test]
fn test_paint_type() {
    let _guard = init_engine();
    let mut shape = Shape::new();
    assert_eq!(shape.paint_type().unwrap(), PaintType::Shape);

    let scene = Scene::new();
    assert_eq!(scene.paint_type().unwrap(), PaintType::Scene);

    let picture = Picture::new();
    assert_eq!(picture.paint_type().unwrap(), PaintType::Picture);

    let text = Text::new();
    assert_eq!(text.paint_type().unwrap(), PaintType::Text);
}

// ── Gradient Properties ────────────────────────────────────────────

#[test]
fn test_linear_gradient_roundtrip() {
    let _guard = init_engine();
    let mut grad = LinearGradient::new();
    grad.set_bounds(10.0, 20.0, 300.0, 400.0).unwrap();
    let (x1, y1, x2, y2) = grad.bounds().unwrap();
    assert!((x1 - 10.0).abs() < f32::EPSILON);
    assert!((y1 - 20.0).abs() < f32::EPSILON);
    assert!((x2 - 300.0).abs() < f32::EPSILON);
    assert!((y2 - 400.0).abs() < f32::EPSILON);
}

#[test]
fn test_radial_gradient_roundtrip() {
    let _guard = init_engine();
    let mut grad = RadialGradient::new();
    grad.set_radial(100.0, 120.0, 50.0, 10.0, 20.0, 5.0)
        .unwrap();
    let (cx, cy, r, fx, fy, fr) = grad.radial().unwrap();
    assert!((cx - 100.0).abs() < f32::EPSILON);
    assert!((cy - 120.0).abs() < f32::EPSILON);
    assert!((r - 50.0).abs() < f32::EPSILON);
    assert!((fx - 10.0).abs() < f32::EPSILON);
    assert!((fy - 20.0).abs() < f32::EPSILON);
    assert!((fr - 5.0).abs() < f32::EPSILON);
}

#[test]
fn test_gradient_spread_roundtrip() {
    let _guard = init_engine();
    let mut grad = LinearGradient::new();
    assert_eq!(grad.spread().unwrap(), FillSpread::Pad);
    grad.set_spread(FillSpread::Reflect).unwrap();
    assert_eq!(grad.spread().unwrap(), FillSpread::Reflect);
    grad.set_spread(FillSpread::Repeat).unwrap();
    assert_eq!(grad.spread().unwrap(), FillSpread::Repeat);
}

#[test]
fn test_gradient_color_stops_roundtrip() {
    let _guard = init_engine();
    let mut grad = LinearGradient::new();
    let stops = [
        ColorStop {
            offset: 0.0,
            r: 10,
            g: 20,
            b: 30,
            a: 40,
        },
        ColorStop {
            offset: 0.5,
            r: 100,
            g: 110,
            b: 120,
            a: 130,
        },
        ColorStop {
            offset: 1.0,
            r: 200,
            g: 210,
            b: 220,
            a: 230,
        },
    ];
    grad.set_color_stops(&stops).unwrap();
    let got = grad.color_stops().unwrap();
    assert_eq!(got.len(), 3);
    for (a, b) in stops.iter().zip(got.iter()) {
        assert!((a.offset - b.offset).abs() < f32::EPSILON);
        assert_eq!(a.r, b.r);
        assert_eq!(a.g, b.g);
        assert_eq!(a.b, b.b);
    }
}

// ── Stroke Dash Roundtrip ──────────────────────────────────────────

#[test]
fn test_stroke_dash_roundtrip() {
    let _guard = init_engine();
    let mut shape = Shape::new();
    shape.set_stroke_width(2.0).unwrap();
    shape.set_stroke_dash(&[10.0, 5.0, 3.0], 2.5).unwrap();

    let (pattern, offset) = shape.stroke_dash().unwrap();
    assert_eq!(pattern.len(), 3);
    assert!((pattern[0] - 10.0).abs() < f32::EPSILON);
    assert!((pattern[1] - 5.0).abs() < f32::EPSILON);
    assert!((pattern[2] - 3.0).abs() < f32::EPSILON);
    assert!((offset - 2.5).abs() < f32::EPSILON);
}

// ── Picture Lifecycle ──────────────────────────────────────────────

#[test]
fn test_picture_create_destroy() {
    let _guard = init_engine();
    let _pic = Picture::new();
    // Dropped without loading — should not crash
}

#[test]
fn test_picture_load_svg_from_memory() {
    let _guard = init_engine();
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\"><rect width=\"100\" height=\"100\" fill=\"red\"/></svg>";
    let mut pic = Picture::new();
    pic.load_data(svg, "svg", None, true).unwrap();
    let (w, h) = pic.size().unwrap();
    assert!(w > 0.0);
    assert!(h > 0.0);
}

// ── Animation Lifecycle ────────────────────────────────────────────

#[test]
fn test_animation_create_destroy() {
    let _guard = init_engine();
    let _anim = Animation::new();
    // Dropped without loading — should not crash
}

// ── Saver Lifecycle ────────────────────────────────────────────────

#[test]
fn test_saver_create_destroy() {
    let _guard = init_engine();
    let _saver = Saver::new();
    // Dropped without saving — should not crash
}

// ── Accessor Lifecycle ─────────────────────────────────────────────

#[test]
fn test_accessor_create_destroy() {
    let _guard = init_engine();
    let _acc = Accessor::new();
}

#[test]
fn test_accessor_generate_id() {
    let _guard = init_engine();
    let id = Accessor::generate_id("test_layer");
    assert!(id.is_some());
    let id2 = Accessor::generate_id("test_layer");
    assert_eq!(id, id2); // Same name → same hash
    let id3 = Accessor::generate_id("other_layer");
    assert_ne!(id, id3); // Different name → different hash
}

// ── Error Handling ─────────────────────────────────────────────────

#[test]
fn test_invalid_picture_load() {
    let _guard = init_engine();
    let mut pic = Picture::new();
    let result = pic.load_from_str("/nonexistent/path.svg");
    assert!(result.is_err());
    // Picture should still be droppable after error
}

#[test]
fn test_shape_stroke_color_without_stroke() {
    let _guard = init_engine();
    let mut shape = Shape::new();
    // Getting stroke color without setting stroke should return error
    let result = shape.stroke_color();
    assert!(result.is_err());
}

// ── Clipping ───────────────────────────────────────────────────────

#[test]
fn test_clip_lifecycle() {
    let _guard = init_engine();

    let mut canvas = SwCanvas::new(EngineOption::Default).unwrap();
    let mut buffer = vec![0u32; 100 * 100];
    unsafe { canvas.set_target(&mut buffer, 100, 100, 100, ColorSpace::ABGR8888) }.unwrap();

    let mut shape = Shape::new();
    shape
        .append_rect(0.0, 0.0, 100.0, 100.0, 0.0, 0.0, true)
        .unwrap();
    shape.set_fill_color(255, 0, 0, 255).unwrap();

    let mut clipper = Shape::new();
    clipper.append_circle(50.0, 50.0, 30.0, 30.0, true).unwrap();
    shape.set_clip(&clipper).unwrap();

    canvas.push(shape).unwrap();
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
    // Clipper and shape are cleaned up properly
}

// ── Full Render Pipeline ───────────────────────────────────────────

#[test]
fn test_full_pipeline_scene_with_effects() {
    let _guard = init_engine();

    let mut canvas = SwCanvas::new(EngineOption::Default).unwrap();
    let mut buffer = vec![0u32; 200 * 200];
    unsafe { canvas.set_target(&mut buffer, 200, 200, 200, ColorSpace::ABGR8888) }.unwrap();

    let mut scene = Scene::new();

    let mut s1 = Shape::new();
    s1.append_rect(10.0, 10.0, 80.0, 80.0, 5.0, 5.0, true)
        .unwrap();
    s1.set_fill_color(255, 0, 0, 255).unwrap();
    scene.push(s1).unwrap();

    let mut s2 = Shape::new();
    s2.append_circle(120.0, 50.0, 30.0, 30.0, true).unwrap();
    s2.set_fill_color(0, 0, 255, 255).unwrap();
    scene.push(s2).unwrap();

    scene.add_gaussian_blur(2.0, 0, 0, 50).unwrap();
    scene.translate(10.0, 10.0).unwrap();

    canvas.push(scene).unwrap();
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();

    assert!(buffer.iter().any(|&px| px != 0));
}
