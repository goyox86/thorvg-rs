extern crate alloc;
extern crate std;

use alloc::vec;
use std::sync::OnceLock;
use crate::*;

/// Shared engine guard — initialized once, kept alive for all tests.
fn init_engine() -> &'static Thorvg {
    static ENGINE: OnceLock<Thorvg> = OnceLock::new();
    ENGINE.get_or_init(|| Thorvg::init(0).expect("Failed to init ThorVG"))
}

#[test]
fn test_init_and_version() {
    let _guard = init_engine();
    let (major, _minor, _micro, version_str) = Thorvg::version().expect("Failed to get version");
    assert!(major >= 1);
    assert!(!version_str.is_empty());
}

#[test]
fn test_canvas_draw_shape() {
    let _guard = init_engine();

    let mut canvas = SwCanvas::new(EngineOption::Default).expect("Failed to create canvas");

    let width = 100u32;
    let height = 100u32;
    let mut buffer = vec![0u32; (width * height) as usize];

    canvas
        .set_target(&mut buffer, width, width, height, ColorSpace::ABGR8888)
        .expect("Failed to set target");

    let mut shape = Shape::new();
    shape
        .append_rect(10.0, 10.0, 50.0, 50.0, 0.0, 0.0, true)
        .unwrap();
    shape.set_fill_color(255, 0, 0, 255).unwrap();

    canvas.push(shape).unwrap();
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();

    assert!(
        buffer.iter().any(|&px| px != 0),
        "Expected non-empty render output"
    );
}

#[test]
fn test_shape_stroke() {
    let _guard = init_engine();

    let mut shape = Shape::new();
    shape.append_circle(50.0, 50.0, 30.0, 30.0, true).unwrap();
    shape.set_stroke_width(3.0).unwrap();
    shape.set_stroke_color(0, 255, 0, 255).unwrap();

    assert!((shape.stroke_width().unwrap() - 3.0).abs() < f32::EPSILON);
}

#[test]
fn test_shape_fill_color() {
    let _guard = init_engine();

    let mut shape = Shape::new();
    shape.set_fill_color(100, 150, 200, 255).unwrap();

    let (r, g, b, a) = shape.fill_color().unwrap();
    assert_eq!((r, g, b, a), (100, 150, 200, 255));
}

#[test]
fn test_paint_opacity() {
    let _guard = init_engine();

    let shape = Shape::new();
    shape.set_opacity(128).unwrap();
    assert_eq!(shape.opacity().unwrap(), 128);
}

#[test]
fn test_linear_gradient() {
    let _guard = init_engine();

    let mut grad = LinearGradient::new();
    grad.set_bounds(0.0, 0.0, 100.0, 100.0).unwrap();
    grad.set_color_stops(&[
        ColorStop { offset: 0.0, r: 255, g: 0, b: 0, a: 255 },
        ColorStop { offset: 1.0, r: 0, g: 0, b: 255, a: 255 },
    ])
    .unwrap();

    let (x1, y1, x2, y2) = grad.bounds().unwrap();
    assert_eq!((x1, y1, x2, y2), (0.0, 0.0, 100.0, 100.0));
}

#[test]
fn test_scene() {
    let _guard = init_engine();

    let mut scene = Scene::new();

    let mut shape1 = Shape::new();
    shape1.append_rect(0.0, 0.0, 50.0, 50.0, 0.0, 0.0, true).unwrap();
    shape1.set_fill_color(255, 0, 0, 255).unwrap();

    let mut shape2 = Shape::new();
    shape2.append_circle(75.0, 75.0, 25.0, 25.0, true).unwrap();
    shape2.set_fill_color(0, 0, 255, 255).unwrap();

    scene.push(shape1).unwrap();
    scene.push(shape2).unwrap();

    let mut canvas = SwCanvas::new(EngineOption::Default).unwrap();
    let mut buffer = vec![0u32; 100 * 100];
    canvas.set_target(&mut buffer, 100, 100, 100, ColorSpace::ABGR8888).unwrap();
    canvas.push(scene).unwrap();
    canvas.draw(true).unwrap();
    canvas.sync().unwrap();

    assert!(buffer.iter().any(|&px| px != 0));
}
