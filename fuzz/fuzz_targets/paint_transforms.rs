#![no_main]

//! Fuzz target for the `Paint` trait surface (transforms, opacity,
//! blend/mask, clip, intersects, bounds, duplicate).
//!
//! Builds a `Shape` and a `Scene` (both implement `Paint`) and
//! drives an arbitrary sequence of trait method calls on the
//! shape, with arbitrary matrix elements, scale factors, rotation
//! angles, opacities, blend/mask methods, and hit-test rectangles.
//! Mask and clip ops consume a freshly-built `Shape` per call to
//! exercise the ownership-transfer paths.

use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use thorvg::{BlendMethod, MaskMethod, Matrix, Paint, Thorvg};

#[derive(Arbitrary, Debug)]
enum Op {
    Scale(f32),
    Rotate(f32),
    Translate(f32, f32),
    SetTransform([f32; 9]),
    SetOpacity(u8),
    SetVisible(bool),
    SetBlend(u8),
    SetMask(u8),
    SetClipRect(f32, f32, f32, f32),
    SetId(u32),
    Intersects(i32, i32, i32, i32),
    QueryBounds,
    QueryBoundsObb,
    QueryTransform,
    QueryVisible,
    QueryOpacity,
    QueryPaintType,
    Duplicate,
}

fn to_blend(b: u8) -> BlendMethod {
    match b % 17 {
        0 => BlendMethod::Normal,
        1 => BlendMethod::Multiply,
        2 => BlendMethod::Screen,
        3 => BlendMethod::Overlay,
        4 => BlendMethod::Darken,
        5 => BlendMethod::Lighten,
        6 => BlendMethod::ColorDodge,
        7 => BlendMethod::ColorBurn,
        8 => BlendMethod::HardLight,
        9 => BlendMethod::SoftLight,
        10 => BlendMethod::Difference,
        11 => BlendMethod::Exclusion,
        12 => BlendMethod::Hue,
        13 => BlendMethod::Saturation,
        14 => BlendMethod::Color,
        15 => BlendMethod::Luminosity,
        _ => BlendMethod::Add,
    }
}

fn to_mask(b: u8) -> MaskMethod {
    match b % 11 {
        0 => MaskMethod::None,
        1 => MaskMethod::Alpha,
        2 => MaskMethod::InvAlpha,
        3 => MaskMethod::Luma,
        4 => MaskMethod::InvLuma,
        5 => MaskMethod::Add,
        6 => MaskMethod::Subtract,
        7 => MaskMethod::Intersect,
        8 => MaskMethod::Difference,
        9 => MaskMethod::Lighten,
        _ => MaskMethod::Darken,
    }
}

#[derive(Debug)]
struct Input {
    ops: Vec<Op>,
}

impl<'a> Arbitrary<'a> for Input {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let n = u.int_in_range::<u16>(0..=128)? as usize;
        let mut ops = Vec::with_capacity(n);
        for _ in 0..n {
            ops.push(Op::arbitrary(u)?);
        }
        Ok(Self { ops })
    }
}

thread_local! {
    static ENGINE: Thorvg = Thorvg::init(0).expect("thorvg init");
}

fuzz_target!(|input: Input| {
    ENGINE.with(|engine| {
        let Ok(mut shape) = engine.shape() else {
            return;
        };
        // Give the shape a non-empty path so AABB/OBB queries have
        // a meaningful interior to operate on.
        let _ = shape.append_rect(0.0, 0.0, 10.0, 10.0, 0.0, 0.0, true);

        for op in input.ops {
            match op {
                Op::Scale(f) => {
                    let _ = shape.scale(f);
                }
                Op::Rotate(d) => {
                    let _ = shape.rotate(d);
                }
                Op::Translate(x, y) => {
                    let _ = shape.translate(x, y);
                }
                Op::SetTransform(m) => {
                    let mtx = Matrix {
                        e11: m[0], e12: m[1], e13: m[2],
                        e21: m[3], e22: m[4], e23: m[5],
                        e31: m[6], e32: m[7], e33: m[8],
                    };
                    let _ = shape.set_transform(&mtx);
                }
                Op::SetOpacity(o) => {
                    let _ = shape.set_opacity(o);
                }
                Op::SetVisible(v) => {
                    let _ = shape.set_visible(v);
                }
                Op::SetBlend(b) => {
                    let _ = shape.set_blend(to_blend(b));
                }
                Op::SetMask(b) => {
                    // set_mask consumes a fresh paint each call.
                    if let Ok(mut m) = engine.shape() {
                        let _ = m.append_rect(0.0, 0.0, 5.0, 5.0, 0.0, 0.0, true);
                        let _ = shape.set_mask(m, to_mask(b));
                    }
                }
                Op::SetClipRect(x, y, w, h) => {
                    if let Ok(mut c) = engine.shape() {
                        let _ = c.append_rect(x, y, w, h, 0.0, 0.0, true);
                        let _ = shape.set_clip(c);
                    }
                }
                Op::SetId(id) => {
                    let _ = shape.set_id(id);
                }
                Op::Intersects(x, y, w, h) => {
                    let _ = shape.intersects(x, y, w, h);
                }
                Op::QueryBounds => {
                    let _ = shape.bounds();
                }
                Op::QueryBoundsObb => {
                    let _ = shape.bounds_obb();
                }
                Op::QueryTransform => {
                    let _ = shape.transform();
                }
                Op::QueryVisible => {
                    let _ = shape.visible();
                }
                Op::QueryOpacity => {
                    let _ = shape.opacity();
                }
                Op::QueryPaintType => {
                    let _ = shape.paint_type();
                }
                Op::Duplicate => {
                    // Duplicate returns an owned new paint; drop
                    // immediately to keep memory bounded.
                    let _ = shape.duplicate();
                }
            }
        }
        // Touch the mask/clip read-side getters after mutation.
        let _ = shape.mask();
        let _ = shape.clip();
        let _ = shape.id();
    });
});
