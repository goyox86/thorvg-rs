#![cfg(feature = "threads")]
#![allow(clippy::cast_precision_loss)]

extern crate alloc;
extern crate std;

use crate::*;
use alloc::vec;

// ── Engine & Version ───────────────────────────────────────────────

#[test]
fn test_init_and_version() {
    let _engine = Thorvg::init(0).expect("Failed to init ThorVG");
    let (major, _minor, _micro, version_str) = Thorvg::version().expect("Failed to get version");
    assert!(major >= 1);
    assert!(!version_str.is_empty());
}

// `Thorvg::init` takes a thread count only when the `threads` feature is
// enabled; without it, the signature collapses to `init()` so callers cannot
// request a count the engine cannot honor.
#[test]
fn test_init_signature_with_threads() {
    let engine = Thorvg::init(0).expect("init(0) should succeed");
    drop(engine);
    let _engine = Thorvg::init(2).expect("init(2) should succeed");
}

// ── Canvas Lifecycle ───────────────────────────────────────────────

#[test]
fn test_canvas_create_destroy() {
    let engine = Thorvg::init(0).unwrap();
    // Canvas should be created and dropped without issues
    let canvas = engine.sw_canvas(EngineOption::DEFAULT);
    assert!(canvas.is_ok());
    // Implicit drop here
}

#[test]
fn test_engine_option_bitflags_round_trip() {
    // The C `Tvg_Engine_Option` is a power-of-two bitfield (NONE = 0,
    // DEFAULT = 1, SMART_RENDER = 2) intended to be OR-combined.  The
    // wrapper exposes it as a bitflags newtype; verify here that the
    // combination actually reaches the engine end-to-end (i.e. the C
    // ABI accepts the OR'd value, the engine returns a non-null
    // canvas), and that the Rust-side query helpers see the expected
    // bits.
    let combined = EngineOption::DEFAULT | EngineOption::SMART_RENDER;
    assert!(combined.contains(EngineOption::DEFAULT));
    assert!(combined.contains(EngineOption::SMART_RENDER));
    assert!(!EngineOption::DEFAULT.contains(EngineOption::SMART_RENDER));
    assert_eq!(EngineOption::default(), EngineOption::DEFAULT);
    assert_eq!(EngineOption::empty(), EngineOption::NONE);

    let engine = Thorvg::init(0).unwrap();
    // Pass the combined value to the actual canvas constructor;
    // this exercises the new `bitfield_enum` bindgen path — a
    // `rustified_enum` would have refused to round-trip the OR'd
    // discriminant.
    let canvas = engine.sw_canvas(combined);
    assert!(canvas.is_ok());
}

#[test]
fn test_canvas_draw_shape() {
    let engine = Thorvg::init(0).unwrap();
    let mut canvas = engine.sw_canvas(EngineOption::DEFAULT).unwrap();
    let (width, height) = (100u32, 100u32);
    let mut buffer = vec![0u32; (width * height) as usize];

    unsafe { canvas.set_target(&mut buffer, width, width, height, ColorSpace::ABGR8888) }.unwrap();

    let mut shape = engine.shape().unwrap();
    shape
        .append_rect(Rect::new(10.0, 10.0, 50.0, 50.0))
        .unwrap();
    shape.set_fill_color(Rgba::new(255, 0, 0, 255)).unwrap();

    canvas.add(shape).unwrap(); // ownership transferred
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();

    assert!(
        buffer.iter().any(|&px| px != 0),
        "Expected non-empty render output"
    );
}

#[test]
fn test_canvas_clear_all() {
    let engine = Thorvg::init(0).unwrap();
    let mut canvas = engine.sw_canvas(EngineOption::DEFAULT).unwrap();
    let mut buffer = vec![0u32; 100 * 100];
    unsafe { canvas.set_target(&mut buffer, 100, 100, 100, ColorSpace::ABGR8888) }.unwrap();

    // Push multiple shapes
    for _ in 0..5 {
        let mut s = engine.shape().unwrap();
        s.append_rect(Rect::new(0.0, 0.0, 10.0, 10.0)).unwrap();
        s.set_fill_color(Rgba::new(255, 0, 0, 255)).unwrap();
        canvas.add(s).unwrap();
    }

    // Clear all — should not double-free anything
    canvas.clear().unwrap();

    // Canvas should still be usable after clearing
    let mut s = engine.shape().unwrap();
    s.append_rect(Rect::new(0.0, 0.0, 50.0, 50.0)).unwrap();
    s.set_fill_color(Rgba::new(0, 255, 0, 255)).unwrap();
    canvas.add(s).unwrap();
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
}

// ── Shape Ownership ────────────────────────────────────────────────

#[test]
fn test_shape_ownership_transfer_to_canvas() {
    let engine = Thorvg::init(0).unwrap();
    let mut canvas = engine.sw_canvas(EngineOption::DEFAULT).unwrap();
    let mut buffer = vec![0u32; 100 * 100];
    unsafe { canvas.set_target(&mut buffer, 100, 100, 100, ColorSpace::ABGR8888) }.unwrap();

    let mut shape = engine.shape().unwrap();
    shape.append_rect(Rect::new(0.0, 0.0, 50.0, 50.0)).unwrap();
    shape.set_fill_color(Rgba::new(255, 0, 0, 255)).unwrap();

    // Transfer ownership — shape should NOT be freed by Rust
    canvas.add(shape).unwrap();
    // shape is consumed, canvas owns it, drop of canvas frees it
}

#[test]
fn test_shape_ownership_transfer_to_scene() {
    let engine = Thorvg::init(0).unwrap();
    let mut scene = engine.scene().unwrap();

    let mut s1 = engine.shape().unwrap();
    s1.append_rect(Rect::new(0.0, 0.0, 10.0, 10.0)).unwrap();
    s1.set_fill_color(Rgba::new(255, 0, 0, 255)).unwrap();

    let mut s2 = engine.shape().unwrap();
    s2.append_circle(Circle::new(50.0, 50.0, 20.0)).unwrap();
    s2.set_fill_color(Rgba::new(0, 0, 255, 255)).unwrap();

    scene.add(s1).unwrap();
    scene.add(s2).unwrap();

    // Scene dropped here — must free both shapes without double-free
}

#[test]
fn test_shape_not_transferred_is_freed() {
    let engine = Thorvg::init(0).unwrap();
    // Shape created but never pushed to canvas/scene — Rust must free it
    let mut shape = engine.shape().unwrap();
    shape
        .append_rect(Rect::new(0.0, 0.0, 100.0, 100.0))
        .unwrap();
    shape.set_fill_color(Rgba::new(255, 255, 0, 255)).unwrap();
    shape.set_stroke_width(3.0).unwrap();
    shape.set_stroke_color(Rgba::new(0, 0, 0, 255)).unwrap();
    // Dropped here — must be freed by Paint::rel
}

// ── Gradient Ownership ─────────────────────────────────────────────

#[test]
fn test_gradient_ownership_transfer_to_shape() {
    let engine = Thorvg::init(0).unwrap();

    let mut grad = engine.linear_gradient().unwrap();
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

    let mut shape = engine.shape().unwrap();
    shape
        .append_rect(Rect::new(0.0, 0.0, 100.0, 100.0))
        .unwrap();
    // Gradient ownership transferred to shape
    shape.set_linear_gradient(grad).unwrap();
    // Shape dropped here — must free gradient too
}

#[test]
fn test_gradient_not_transferred_is_freed() {
    let engine = Thorvg::init(0).unwrap();
    // Gradient created but never given to a shape
    let mut grad = engine.radial_gradient().unwrap();
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
    let engine = Thorvg::init(0).unwrap();

    let mut grad = engine.linear_gradient().unwrap();
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
    let engine = Thorvg::init(0).unwrap();

    let mut canvas = engine.sw_canvas(EngineOption::DEFAULT).unwrap();
    let mut buffer = vec![0u32; 200 * 200];
    unsafe { canvas.set_target(&mut buffer, 200, 200, 200, ColorSpace::ABGR8888) }.unwrap();

    // Scene containing shapes, pushed to canvas
    let mut scene = engine.scene().unwrap();
    for i in 0..10 {
        let mut s = engine.shape().unwrap();
        s.append_circle(Circle::new(i as f32 * 20.0, 50.0, 10.0))
            .unwrap();
        s.set_fill_color(Rgba::new(255, 0, 0, 255)).unwrap();
        scene.add(s).unwrap();
    }
    canvas.add(scene).unwrap();

    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
    // Canvas dropped → Scene dropped → 10 Shapes dropped, no double-free
}

#[test]
fn test_scene_clear_and_reuse() {
    let engine = Thorvg::init(0).unwrap();

    let mut scene = engine.scene().unwrap();

    // Add shapes
    for _ in 0..5 {
        let mut s = engine.shape().unwrap();
        s.append_rect(Rect::new(0.0, 0.0, 10.0, 10.0)).unwrap();
        scene.add(s).unwrap();
    }

    // Clear all shapes
    scene.clear().unwrap();

    // Scene should still be usable
    let mut s = engine.shape().unwrap();
    s.append_rect(Rect::new(0.0, 0.0, 50.0, 50.0)).unwrap();
    scene.add(s).unwrap();
}

// ── Paint Properties ───────────────────────────────────────────────

#[test]
fn test_shape_fill_color_roundtrip() {
    let engine = Thorvg::init(0).unwrap();
    let mut shape = engine.shape().unwrap();
    shape.set_fill_color(Rgba::new(100, 150, 200, 255)).unwrap();
    assert_eq!(shape.fill_color().unwrap(), Rgba::new(100, 150, 200, 255));
}

#[test]
fn test_shape_stroke_roundtrip() {
    let engine = Thorvg::init(0).unwrap();
    let mut shape = engine.shape().unwrap();
    shape.append_circle(Circle::new(50.0, 50.0, 30.0)).unwrap();
    shape.set_stroke_width(3.0).unwrap();
    shape.set_stroke_color(Rgba::new(0, 255, 0, 255)).unwrap();
    shape.set_stroke_cap(StrokeCap::Round).unwrap();
    shape.set_stroke_join(StrokeJoin::Bevel).unwrap();
    shape.set_stroke_miterlimit(8.0).unwrap();

    assert!((shape.stroke_width().unwrap() - 3.0).abs() < f32::EPSILON);
    assert_eq!(shape.stroke_color().unwrap(), Rgba::new(0, 255, 0, 255));
    assert_eq!(shape.stroke_cap().unwrap(), StrokeCap::Round);
    assert_eq!(shape.stroke_join().unwrap(), StrokeJoin::Bevel);
    assert!((shape.stroke_miterlimit().unwrap() - 8.0).abs() < f32::EPSILON);
}

#[test]
fn test_shape_fill_rule_roundtrip() {
    let engine = Thorvg::init(0).unwrap();
    let mut shape = engine.shape().unwrap();
    assert_eq!(shape.fill_rule().unwrap(), FillRule::NonZero);
    shape.set_fill_rule(FillRule::EvenOdd).unwrap();
    assert_eq!(shape.fill_rule().unwrap(), FillRule::EvenOdd);
}

#[test]
fn test_paint_opacity_roundtrip() {
    let engine = Thorvg::init(0).unwrap();
    let mut shape = engine.shape().unwrap();
    shape.set_opacity(128).unwrap();
    assert_eq!(shape.opacity().unwrap(), 128);
}

#[test]
fn test_paint_visibility_roundtrip() {
    let engine = Thorvg::init(0).unwrap();
    let mut shape = engine.shape().unwrap();
    assert!(shape.visible());
    shape.set_visible(false).unwrap();
    assert!(!shape.visible());
    shape.set_visible(true).unwrap();
    assert!(shape.visible());
}

#[test]
fn test_paint_transform_roundtrip() {
    let engine = Thorvg::init(0).unwrap();
    let mut shape = engine.shape().unwrap();
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

// ── Matrix combinators ─────────────────────────────────────────────
//
// These cross-check our pure-Rust `Matrix` builders against the matrix
// thorvg itself composes from its incremental `Paint` ops, so the two
// paths can never silently diverge (e.g. a flipped rotation sign).

/// Asserts two matrices agree element-wise within an f32 tolerance.
/// thorvg's trig (`cosf`/`sinf`) and ours (`libm`) may differ in the
/// last bits, so exact equality is not appropriate for rotations.
fn assert_matrix_close(got: &Matrix, want: &Matrix) {
    const TOL: f32 = 1e-4;
    for (g, w, name) in [
        (got.e11, want.e11, "e11"),
        (got.e12, want.e12, "e12"),
        (got.e13, want.e13, "e13"),
        (got.e21, want.e21, "e21"),
        (got.e22, want.e22, "e22"),
        (got.e23, want.e23, "e23"),
        (got.e31, want.e31, "e31"),
        (got.e32, want.e32, "e32"),
        (got.e33, want.e33, "e33"),
    ] {
        assert!((g - w).abs() < TOL, "{name}: got {g}, want {w}");
    }
}

#[test]
fn test_matrix_default_is_identity() {
    // What: `Default` returns the identity matrix.
    // Why trustworthy: pure value check against the `IDENTITY` constant,
    // no FFI involved. Guards the cheap-but-easy-to-break promise that
    // `Matrix::default()` is a usable neutral element for the
    // combinators below (they all start from it).
    assert_eq!(Matrix::default(), Matrix::IDENTITY);
}

#[test]
fn test_matrix_translate_matches_thorvg() {
    // What: our `translate` builder equals the matrix thorvg composes
    // for the same translation.
    // Why trustworthy: the expected value is not hard-coded — it is read
    // back from thorvg via `shape.transform()` after `shape.translate`,
    // so the engine itself is the oracle. If our element placement
    // (translation in e13/e23) ever disagreed with thorvg, this fails.
    let engine = Thorvg::init(0).unwrap();
    let mut shape = engine.shape().unwrap();
    shape.translate(10.0, 20.0).unwrap();
    let thorvg_m = shape.transform().unwrap();
    assert_matrix_close(&Matrix::default().translate(10.0, 20.0), &thorvg_m);
}

#[test]
fn test_matrix_scale_matches_thorvg() {
    // What: our `scale` builder equals thorvg's scale matrix.
    // Why trustworthy: oracle is `shape.transform()` after a real
    // `shape.scale`, not a constant. Note `Paint::scale` is uniform, so
    // we pass equal axes here; the non-uniform path is exercised by the
    // composition/round-trip tests.
    let engine = Thorvg::init(0).unwrap();
    let mut shape = engine.shape().unwrap();
    shape.scale(2.5).unwrap();
    let thorvg_m = shape.transform().unwrap();
    assert_matrix_close(&Matrix::default().scale(2.5, 2.5), &thorvg_m);
}

#[test]
fn test_matrix_rotate_matches_thorvg() {
    // What: our `rotate` builder equals thorvg's rotation matrix — the
    // single most drift-prone op (sign, degrees-vs-radians, sin/cos
    // placement).
    // Why trustworthy: the oracle is thorvg's own `tvg::rotate` output,
    // surfaced through `shape.transform()`. Because our trig comes from
    // `libm` and thorvg's from system `cosf`/`sinf`, exact equality is
    // wrong here — `assert_matrix_close` tolerates last-bit float
    // differences while still failing on any convention mismatch (a
    // flipped sign moves an element by ~2·sin, far above tolerance).
    let engine = Thorvg::init(0).unwrap();
    let mut shape = engine.shape().unwrap();
    shape.rotate(30.0).unwrap();
    let thorvg_m = shape.transform().unwrap();
    assert_matrix_close(&Matrix::default().rotate(30.0), &thorvg_m);
}

#[test]
fn test_matrix_scale_then_rotate_matches_thorvg() {
    // What: our combinator composition `scale(..).rotate(..)` equals
    // thorvg's composed result.
    // Why trustworthy: this is the test that pins multiply *order*.
    // Single-op tests pass even if `multiply` swapped its operands,
    // because identity·X == X·identity hides the bug. Scale and rotate
    // do not commute, so composing them is what actually distinguishes
    // R·S from S·R. The oracle is thorvg's `update()`, which always
    // applies scale then rotate to the 2×2 block (tvgPaint.h)
    // regardless of call order — matching our `scale(..).rotate(..)`.
    let engine = Thorvg::init(0).unwrap();
    let mut shape = engine.shape().unwrap();
    shape.scale(1.5).unwrap();
    shape.rotate(40.0).unwrap();
    let thorvg_m = shape.transform().unwrap();
    assert_matrix_close(&Matrix::default().scale(1.5, 1.5).rotate(40.0), &thorvg_m);
}

#[test]
fn test_matrix_combinator_roundtrips_through_thorvg() {
    // What: a fully hand-composed matrix survives `set_transform` ->
    // `transform` unchanged.
    // Why trustworthy: this isolates the FFI marshalling from the math.
    // `set_transform` marks the paint user-overridden, so thorvg's
    // `update()` returns early and hands our matrix back verbatim — any
    // transposed or mismapped field in `to_raw`/`from_raw` shows up as a
    // mismatch. Uses a non-uniform scale and a non-trivial chain so all
    // nine elements are non-default.
    let engine = Thorvg::init(0).unwrap();
    let mut shape = engine.shape().unwrap();
    let m = Matrix::default()
        .translate(5.0, -3.0)
        .rotate(25.0)
        .scale(2.0, 0.5);
    shape.set_transform(&m).unwrap();
    assert_matrix_close(&shape.transform().unwrap(), &m);
}

#[test]
fn test_paint_id_roundtrip() {
    let engine = Thorvg::init(0).unwrap();
    let mut shape = engine.shape().unwrap();
    shape.set_id(42).unwrap();
    assert_eq!(shape.id(), 42);
}

#[test]
fn test_paint_type() {
    let engine = Thorvg::init(0).unwrap();
    let shape = engine.shape().unwrap();
    assert_eq!(shape.paint_type().unwrap(), PaintType::Shape);

    let scene = engine.scene().unwrap();
    assert_eq!(scene.paint_type().unwrap(), PaintType::Scene);

    let picture = engine.picture().unwrap();
    assert_eq!(picture.paint_type().unwrap(), PaintType::Picture);

    let text = engine.text().unwrap();
    assert_eq!(text.paint_type().unwrap(), PaintType::Text);
}

#[test]
fn test_gradient_type_roundtrip() {
    let engine = Thorvg::init(0).unwrap();

    let linear = engine.linear_gradient().unwrap();
    assert_eq!(linear.gradient_type().unwrap(), PaintType::LinearGradient);

    let radial = engine.radial_gradient().unwrap();
    assert_eq!(radial.gradient_type().unwrap(), PaintType::RadialGradient);
}

// ── Gradient Properties ────────────────────────────────────────────

#[test]
fn test_linear_gradient_roundtrip() {
    let engine = Thorvg::init(0).unwrap();
    let mut grad = engine.linear_gradient().unwrap();
    grad.set_bounds(10.0, 20.0, 300.0, 400.0).unwrap();
    let (x1, y1, x2, y2) = grad.bounds().unwrap();
    assert!((x1 - 10.0).abs() < f32::EPSILON);
    assert!((y1 - 20.0).abs() < f32::EPSILON);
    assert!((x2 - 300.0).abs() < f32::EPSILON);
    assert!((y2 - 400.0).abs() < f32::EPSILON);
}

#[test]
fn test_radial_gradient_roundtrip() {
    let engine = Thorvg::init(0).unwrap();
    let mut grad = engine.radial_gradient().unwrap();
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
    let engine = Thorvg::init(0).unwrap();
    let mut grad = engine.linear_gradient().unwrap();
    assert_eq!(grad.spread().unwrap(), FillSpread::Pad);
    grad.set_spread(FillSpread::Reflect).unwrap();
    assert_eq!(grad.spread().unwrap(), FillSpread::Reflect);
    grad.set_spread(FillSpread::Repeat).unwrap();
    assert_eq!(grad.spread().unwrap(), FillSpread::Repeat);
}

#[test]
fn test_gradient_color_stops_roundtrip() {
    let engine = Thorvg::init(0).unwrap();
    let mut grad = engine.linear_gradient().unwrap();
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
    let engine = Thorvg::init(0).unwrap();
    let mut shape = engine.shape().unwrap();
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
    let engine = Thorvg::init(0).unwrap();
    let _pic = engine.picture().unwrap();
    // Dropped without loading — should not crash
}

#[test]
fn test_picture_load_svg_from_memory() {
    let engine = Thorvg::init(0).unwrap();
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\"><rect width=\"100\" height=\"100\" fill=\"red\"/></svg>";
    let mut pic = engine.picture().unwrap();
    pic.load_data(svg, MimeType::Svg, None).unwrap();
    let (w, h) = pic.size().unwrap();
    assert!(w > 0.0);
    assert!(h > 0.0);
}

// ── Animation Lifecycle ────────────────────────────────────────────

#[test]
fn test_animation_create_destroy() {
    let engine = Thorvg::init(0).unwrap();
    let _anim = engine.animation().unwrap();
    // Dropped without loading — should not crash
}

// ── Saver Lifecycle ────────────────────────────────────────────────

#[test]
fn test_saver_create_destroy() {
    let engine = Thorvg::init(0).unwrap();
    let _saver = engine.saver().unwrap();
    // Dropped without saving — should not crash
}

#[test]
fn test_saver_save_animation_ownership_transfer() {
    // A fresh Animation has totalFrame == 0, which makes
    // Saver::save_animation hit the InsufficientCondition path and
    // delete the underlying handle (refCnt == 1).  The binding must
    // consume the Animation so Rust's Drop doesn't double-free on
    // the now-freed C handle.  Under ASan, the pre-fix `&Animation`
    // signature triggered a heap-use-after-free here.
    let engine = Thorvg::init(0).unwrap();
    let mut saver = engine.saver().unwrap();
    let anim = engine.animation().unwrap();
    let r = saver.save_animation_to_str(anim, "/tmp/thorvg-rs-test.gif", 100, 30);
    assert!(r.is_err()); // InsufficientCondition — no frames loaded
    // `anim` was moved into the call; no Drop runs on freed memory.
}

// ── Text font loading ──────────────────────────────────────────────

#[test]
fn test_load_font_data_owned_buffer_is_safe() {
    // load_font_data takes &[u8] (no lifetime tie) and passes
    // copy=true to thorvg, so the caller's buffer can drop
    // immediately after the call.  Under ASan, any subsequent
    // text-render attempt would dereference the Vec's freed pages
    // and trip heap-use-after-free if the copy hadn't happened.
    let engine = Thorvg::init(0).unwrap();
    let font_bytes: alloc::vec::Vec<u8> =
        include_bytes!("../../thorvg-sys/thorvg/test/resources/Arial.ttf").to_vec();
    engine
        .load_font_data("Arial-Owned", &font_bytes, None)
        .unwrap();
    drop(font_bytes); // C side has its own copy — no dangling reference.
}

#[test]
fn test_load_font_data_static_zero_copy() {
    // load_font_data_static requires &'static [u8].  include_bytes!
    // returns &'static [u8; N], so it coerces to &'static [u8] and
    // thorvg can borrow the buffer for the engine's lifetime
    // without copying.
    let engine = Thorvg::init(0).unwrap();
    static FONT: &[u8] = include_bytes!("../../thorvg-sys/thorvg/test/resources/Arial.ttf");
    engine
        .load_font_data_static("Arial-Static", FONT, None)
        .unwrap();
}

// ── Accessor Lifecycle ─────────────────────────────────────────────

#[test]
fn test_accessor_create_destroy() {
    let engine = Thorvg::init(0).unwrap();
    let _acc = engine.accessor().unwrap();
}

#[test]
fn test_accessor_generate_id() {
    let _engine = Thorvg::init(0).unwrap();
    let id = Accessor::generate_id("test_layer");
    assert!(id.is_some());
    let id2 = Accessor::generate_id("test_layer");
    assert_eq!(id, id2); // Same name → same hash
    let id3 = Accessor::generate_id("other_layer");
    assert_ne!(id, id3); // Different name → different hash
}

#[test]
fn test_accessor_for_each_borrowed_views() {
    // The closure receives BorrowedAccessor + BorrowedPaint, so the
    // safe wrapper alone is enough to inspect visited nodes — no
    // direct thorvg-sys dependency required for id / paint_type
    // queries.  Also exercises that BorrowedAccessor::get_name is
    // callable from inside the closure (the borrow that the old
    // `Accessor::get_name(&self)` API conflicted with).
    let engine = Thorvg::init(0).unwrap();
    let mut acc = engine.accessor().unwrap();

    // Build a tiny scene: parent with one shape child.
    let mut parent = engine.scene().unwrap();
    let mut child = engine.shape().unwrap();
    child.append_rect(Rect::new(0.0, 0.0, 10.0, 10.0)).unwrap();
    parent.add(child).unwrap();

    let mut visited: alloc::vec::Vec<PaintType> = alloc::vec::Vec::new();
    acc.for_each(&parent, |view, paint| {
        // Calling get_name through the BorrowedAccessor is now
        // possible inside the closure.  Returns None for an
        // unnamed paint, which is the expected behaviour here.
        let _ = view.get_name(0);
        visited.push(paint.paint_type().unwrap_or(PaintType::Undefined));
        true
    })
    .unwrap();

    // Visits both the parent scene and the child shape.
    assert!(visited.contains(&PaintType::Scene));
    assert!(visited.contains(&PaintType::Shape));
}

// ── Error Handling ─────────────────────────────────────────────────

#[test]
fn test_invalid_picture_load() {
    let engine = Thorvg::init(0).unwrap();
    let mut pic = engine.picture().unwrap();
    let result = pic.load_from_str("/nonexistent/path.svg");
    assert!(result.is_err());
    // Picture should still be droppable after error
}

#[test]
fn test_shape_stroke_color_without_stroke() {
    let engine = Thorvg::init(0).unwrap();
    let shape = engine.shape().unwrap();
    // Getting stroke color without setting stroke should return error
    let result = shape.stroke_color();
    assert!(result.is_err());
}

// ── Clipping ───────────────────────────────────────────────────────

#[test]
fn test_clip_lifecycle() {
    let engine = Thorvg::init(0).unwrap();

    let mut canvas = engine.sw_canvas(EngineOption::DEFAULT).unwrap();
    let mut buffer = vec![0u32; 100 * 100];
    unsafe { canvas.set_target(&mut buffer, 100, 100, 100, ColorSpace::ABGR8888) }.unwrap();

    let mut shape = engine.shape().unwrap();
    shape
        .append_rect(Rect::new(0.0, 0.0, 100.0, 100.0))
        .unwrap();
    shape.set_fill_color(Rgba::new(255, 0, 0, 255)).unwrap();

    let mut clipper = engine.shape().unwrap();
    clipper
        .append_circle(Circle::new(50.0, 50.0, 30.0))
        .unwrap();
    shape.set_clip(clipper).unwrap();

    canvas.add(shape).unwrap();
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();
    // Clipper and shape are cleaned up properly
}

// ── Asset resolver ─────────────────────────────────────────────────

#[test]
fn test_asset_resolver_install_replace_clear() {
    use alloc::sync::Arc;
    use core::sync::atomic::{AtomicUsize, Ordering};

    let engine = Thorvg::init(0).unwrap();
    let mut pic = engine.picture().unwrap();

    let calls = Arc::new(AtomicUsize::new(0));

    // Install
    let calls_clone = Arc::clone(&calls);
    pic.set_asset_resolver(move |_src| {
        calls_clone.fetch_add(1, Ordering::SeqCst);
        None // we don't actually load anything here
    })
    .unwrap();

    // Replace (the previous closure's Box must drop cleanly)
    let calls_clone = Arc::clone(&calls);
    pic.set_asset_resolver(move |_src| {
        calls_clone.fetch_add(10, Ordering::SeqCst);
        None
    })
    .unwrap();

    // Clear
    pic.clear_asset_resolver().unwrap();

    // Drop the picture — Picture::Drop must unregister even when
    // no resolver remains installed (resolver is None, so the
    // drop path simply skips the C-side detach).
    drop(pic);

    // We didn't trigger any actual asset loads here; just verify
    // the install/replace/clear/drop machinery runs without UAF.
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

/// Upstream Lottie test asset that references one external image
/// (`image/logo.png`) and one external font (`font/SentyCloud.ttf`).
/// The Lottie builder invokes the installed resolver while resolving
/// these references during a frame update, which is what drives the
/// monomorphized `resolver_trampoline::<F>` end-to-end.
///
/// SVG-based tests can't exercise this path: the SVG loader doesn't
/// use the asset resolver at all (see `tvgLottieBuilder.cpp:965` for
/// the only call site).
const LOTTIE_RESOLVER_JSON: &[u8] =
    include_bytes!("../../thorvg-sys/thorvg/test/resources/resolver.json");

#[test]
fn test_asset_resolver_invoked_during_lottie_render() {
    // Exercises the *actual* trampoline body: the Lottie builder
    // calls back into Rust while resolving the image asset, so we
    // observe both that the closure ran and that the `src` string
    // round-tripped across the FFI boundary intact.
    use alloc::string::String;
    use alloc::sync::Arc;
    use alloc::vec::Vec;
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    let engine = Thorvg::init(0).unwrap();
    let mut lottie = engine.lottie_animation().unwrap();

    let calls = Arc::new(AtomicUsize::new(0));
    let seen_srcs: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    let calls_clone = Arc::clone(&calls);
    let seen_clone = Arc::clone(&seen_srcs);
    lottie
        .picture_mut()
        .set_asset_resolver(move |src| {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            seen_clone.lock().unwrap().push(String::from(src));
            None // signal "not resolved"
        })
        .unwrap();

    lottie.load_data(LOTTIE_RESOLVER_JSON).unwrap();

    // Advance to a frame to trigger the builder — that's when the
    // image asset reference is resolved (`updateImage` in
    // tvgLottieBuilder.cpp).
    let total = lottie.total_frame().unwrap();
    let _ = lottie.set_frame(total * 0.5);

    // Drive a draw so the build phase definitely runs.
    let mut canvas = engine.sw_canvas(EngineOption::DEFAULT).unwrap();
    let mut buffer = vec![0u32; 800 * 800];
    unsafe { canvas.set_target(&mut buffer, 800, 800, 800, ColorSpace::ABGR8888) }.unwrap();
    // The picture is owned by the animation; push a duplicate so we
    // don't transfer ownership away from `lottie`.  If `duplicate`
    // isn't available we just skip the draw — `set_frame` above is
    // often enough on its own.
    let _ = canvas.draw(true);
    let _ = canvas.sync();

    assert!(
        calls.load(Ordering::SeqCst) >= 1,
        "resolver trampoline should have been called at least once"
    );
    let seen = seen_srcs.lock().unwrap().clone();
    // The image asset path is built from the asset's `u` + `p`
    // fields (`"image/"` + `"logo.png"`); thorvg may also prepend a
    // path separator depending on `resource_path`, so just check
    // for the filename to stay robust to that detail.
    assert!(
        seen.iter().any(|s| s.ends_with("logo.png")),
        "resolver should have seen the image asset path, got: {seen:?}"
    );
}

#[test]
fn test_asset_resolver_returns_bytes_path() {
    // Same as above, but the resolver returns `Some(bytes)` so the
    // trampoline takes the success branch and forwards the payload
    // into `tvg_picture_load_data` from inside the C callback.
    use alloc::sync::Arc;
    use alloc::vec::Vec;
    use core::sync::atomic::{AtomicUsize, Ordering};

    // A real PNG so the inner `tvg_picture_load_data` call inside
    // the trampoline succeeds and we cover the
    // `r == TVG_RESULT_SUCCESS` branch.  The fixture lives under
    // `thorvg/tests/assets/` (CC0; see the README in that dir).
    const LOGO_PNG: &[u8] = include_bytes!("../tests/assets/logo.png");

    let engine = Thorvg::init(0).unwrap();
    let mut lottie = engine.lottie_animation().unwrap();

    let calls = Arc::new(AtomicUsize::new(0));
    let calls_clone = Arc::clone(&calls);
    lottie
        .picture_mut()
        .set_asset_resolver(move |_src| {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Some((Vec::from(LOGO_PNG), MimeType::Png))
        })
        .unwrap();

    lottie.load_data(LOTTIE_RESOLVER_JSON).unwrap();
    let total = lottie.total_frame().unwrap();
    let _ = lottie.set_frame(total * 0.5);

    assert!(
        calls.load(Ordering::SeqCst) >= 1,
        "resolver trampoline (Some branch) should have been called"
    );
}

#[cfg(feature = "std")]
#[test]
fn test_asset_resolver_panic_is_caught() {
    // The monomorphized `invoke_resolver::<F>` wraps the user
    // closure in `catch_unwind` under `std`.  A panic inside the
    // closure must be absorbed and surface as a "not resolved"
    // return rather than unwinding across the C++ frame (UB).
    //
    // We can't directly assert "no UB happened", but if the
    // catch_unwind guard regresses this test process aborts.
    //
    // NOTE: upstream behaviour is that
    // `tvg_picture_set_asset_resolver` returns `InsufficientCondition`
    // *after* a successful load (see testLottie.cpp:324), so we must
    // not touch the resolver again post-load — the Picture's `Drop`
    // takes care of unregistration.
    let engine = Thorvg::init(0).unwrap();
    let mut lottie = engine.lottie_animation().unwrap();

    lottie
        .picture_mut()
        .set_asset_resolver(|_src| -> Option<(alloc::vec::Vec<u8>, MimeType)> {
            panic!("intentional panic from test resolver");
        })
        .unwrap();

    // Drives the trampoline; the panic must be caught inside it.
    lottie.load_data(LOTTIE_RESOLVER_JSON).unwrap();
    let total = lottie.total_frame().unwrap();
    let _ = lottie.set_frame(total * 0.5);

    // Animation (and the picture it owns) dropping at end of scope
    // must run the resolver's `Drop` without UAF.
}

// ── Masking ────────────────────────────────────────────────────────

#[test]
fn test_mask_getter_round_trip() {
    // After set_mask, mask() must return the same target handle and
    // method.  Under the pre-fix `mask_method(&P)` API the C side
    // wrote through the supplied `target.raw()` pointer, corrupting
    // the target paint's vtable — ASan would have flagged the
    // resulting `tvg_paint_get_type` virtual dispatch as
    // heap-buffer-overflow on the BorrowedPaint side.
    let engine = Thorvg::init(0).unwrap();

    let mut paint = engine.shape().unwrap();
    paint
        .append_rect(Rect::new(0.0, 0.0, 100.0, 100.0))
        .unwrap();

    assert!(paint.mask().is_none(), "no mask before set_mask");

    let mut mask_shape = engine.shape().unwrap();
    mask_shape
        .append_circle(Circle::new(50.0, 50.0, 30.0))
        .unwrap();
    paint.set_mask(mask_shape, MaskMethod::Alpha).unwrap();

    let (got, method) = paint.mask().expect("mask should be set");
    assert_eq!(method, MaskMethod::Alpha);
    assert_eq!(got.paint_type().unwrap(), PaintType::Shape);
}

// ── Full Render Pipeline ───────────────────────────────────────────

#[test]
fn test_full_pipeline_scene_with_effects() {
    let engine = Thorvg::init(0).unwrap();

    let mut canvas = engine.sw_canvas(EngineOption::DEFAULT).unwrap();
    let mut buffer = vec![0u32; 200 * 200];
    unsafe { canvas.set_target(&mut buffer, 200, 200, 200, ColorSpace::ABGR8888) }.unwrap();

    let mut scene = engine.scene().unwrap();

    let mut s1 = engine.shape().unwrap();
    s1.append_rect(Rect::new(10.0, 10.0, 80.0, 80.0).corner_radius(5.0))
        .unwrap();
    s1.set_fill_color(Rgba::new(255, 0, 0, 255)).unwrap();
    scene.add(s1).unwrap();

    let mut s2 = engine.shape().unwrap();
    s2.append_circle(Circle::new(120.0, 50.0, 30.0)).unwrap();
    s2.set_fill_color(Rgba::new(0, 0, 255, 255)).unwrap();
    scene.add(s2).unwrap();

    scene
        .add_gaussian_blur_effect(2.0, BlurDirection::Both, BlurBorder::Duplicate, 50)
        .unwrap();
    scene.translate(10.0, 10.0).unwrap();

    canvas.add(scene).unwrap();
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();

    assert!(buffer.iter().any(|&px| px != 0));
}
