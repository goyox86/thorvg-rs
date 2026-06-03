#![no_main]

//! Fuzz target for `Shape::append_path`.
//!
//! `cmds: &[u8]` is interpreted as path opcodes by the C path
//! builder (`MoveTo` = 1 point, `LineTo` = 1 point, `CubicTo` = 3
//! points, `Close` = 0 points).  A mismatch between the opcode
//! arities and `pts.len()`, or an unrecognised opcode, can drive the
//! C reader past the end of `pts`.  Feeding arbitrary
//! `(cmds, pts)` exercises that boundary.

use libfuzzer_sys::arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use thorvg::{Point, Thorvg};

#[derive(Arbitrary, Debug)]
struct PtIn {
    x: f32,
    y: f32,
}

#[derive(Arbitrary, Debug)]
struct Input {
    cmds: Vec<u8>,
    pts: Vec<PtIn>,
}

thread_local! {
    static ENGINE: Thorvg = Thorvg::init(0).expect("thorvg init");
}

fuzz_target!(|input: Input| {
    ENGINE.with(|engine| {
        let Ok(mut shape) = engine.shape() else {
            return;
        };
        let pts: Vec<Point> = input.pts.iter().map(|p| Point { x: p.x, y: p.y }).collect();
        let _ = shape.append_path(&input.cmds, &pts);
    });
});
