#![no_main]

//! Fuzz target for the `Shape` path/style builders.
//!
//! Drives an arbitrary sequence of `Shape` mutator calls — path
//! primitives (`move_to`, `line_to`, `cubic_to`, `close`,
//! `append_rect`, `append_circle`), stroke/fill setters
//! (`set_stroke_width`, `set_stroke_cap`, `set_stroke_join`,
//! `set_stroke_miterlimit`, `set_stroke_dash`, `set_fill_color`,
//! `set_stroke_color`, `set_fill_rule`, `set_paint_order`), plus
//! `set_trimpath` and `reset`.  All numeric inputs are arbitrary
//! floats (NaN, inf, denormals) so any wrapper-side coordinate
//! validation or C-side path-state machine that mis-handles a
//! pathological sequence surfaces as a crash.

use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use thorvg::{Circle, FillRule, Rect, Rgba, StrokeCap, StrokeJoin, Thorvg};

#[derive(Arbitrary, Debug)]
enum Op {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    CubicTo(f32, f32, f32, f32, f32, f32),
    Close,
    AppendRect(f32, f32, f32, f32, f32, f32, bool),
    AppendCircle(f32, f32, f32, f32, bool),
    SetFillColor(u8, u8, u8, u8),
    SetStrokeColor(u8, u8, u8, u8),
    SetFillRule(u8),
    SetStrokeWidth(f32),
    SetStrokeCap(u8),
    SetStrokeJoin(u8),
    SetStrokeMiterlimit(f32),
    SetStrokeDash(Vec<f32>, f32),
    SetTrimpath(f32, f32, bool),
    SetPaintOrder(bool),
    Reset,
}

fn to_fill_rule(b: u8) -> FillRule {
    match b & 1 {
        0 => FillRule::NonZero,
        _ => FillRule::EvenOdd,
    }
}

fn to_cap(b: u8) -> StrokeCap {
    match b % 3 {
        0 => StrokeCap::Square,
        1 => StrokeCap::Round,
        _ => StrokeCap::Butt,
    }
}

fn to_join(b: u8) -> StrokeJoin {
    match b % 3 {
        0 => StrokeJoin::Bevel,
        1 => StrokeJoin::Round,
        _ => StrokeJoin::Miter,
    }
}

#[derive(Debug)]
struct Input {
    ops: Vec<Op>,
}

impl<'a> Arbitrary<'a> for Input {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        // Cap op count so a single iteration stays bounded.
        let n = u.int_in_range::<u16>(0..=256)? as usize;
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
        for op in input.ops {
            let _ = match op {
                Op::MoveTo(x, y) => shape.move_to(x, y),
                Op::LineTo(x, y) => shape.line_to(x, y),
                Op::CubicTo(a, b, c, d, e, f) => shape.cubic_to(a, b, c, d, e, f),
                Op::Close => shape.close(),
                Op::AppendRect(x, y, w, h, rx, ry, cw) => shape.append_rect(Rect {
                    x,
                    y,
                    width: w,
                    height: h,
                    rx,
                    ry,
                    cw,
                }),
                Op::AppendCircle(cx, cy, rx, ry, cw) => {
                    shape.append_circle(Circle { cx, cy, rx, ry, cw })
                }
                Op::SetFillColor(r, g, b, a) => shape.set_fill_color(Rgba::new(r, g, b, a)),
                Op::SetStrokeColor(r, g, b, a) => shape.set_stroke_color(Rgba::new(r, g, b, a)),
                Op::SetFillRule(b) => shape.set_fill_rule(to_fill_rule(b)),
                Op::SetStrokeWidth(w) => shape.set_stroke_width(w),
                Op::SetStrokeCap(b) => shape.set_stroke_cap(to_cap(b)),
                Op::SetStrokeJoin(b) => shape.set_stroke_join(to_join(b)),
                Op::SetStrokeMiterlimit(m) => shape.set_stroke_miterlimit(m),
                Op::SetStrokeDash(pattern, offset) => shape.set_stroke_dash(&pattern, offset),
                Op::SetTrimpath(b, e, sim) => shape.set_trimpath(b, e, sim),
                Op::SetPaintOrder(stroke_first) => shape.set_paint_order(stroke_first),
                Op::Reset => shape.reset(),
            };
        }
        // Exercise the read-side path-counter getter at the end of
        // a (potentially malformed) sequence.
        let _ = shape.path();
    });
});
