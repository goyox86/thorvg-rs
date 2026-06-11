#![no_main]

//! Fuzz target for `Shape::append_path`.
//!
//! The Rust API now takes a typed [`Path`] (`commands: Vec<PathCommand>`,
//! `points: Vec<Point>`), so unknown opcode bytes are no longer
//! reachable through the wrapper.  The remaining failure surface is
//! **arity mismatches**: a `MoveTo` / `LineTo` / `CubicTo` declared
//! with too few accompanying points would drive the C path builder
//! past the end of `points` if the engine didn't check.  Feeding
//! arbitrary, independently-generated command and point vectors
//! exercises that boundary.

use libfuzzer_sys::arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use thorvg::{Path, PathCommand, Point, Thorvg};

#[derive(Arbitrary, Debug)]
struct PtIn {
    x: f32,
    y: f32,
}

/// Arbitrary command kind: a `u8` that maps to one of the four
/// `PathCommand` variants via `% 4`, so the fuzzer's byte stream
/// reaches every variant with even probability.
#[derive(Arbitrary, Debug, Clone, Copy)]
struct CmdIn(u8);

impl CmdIn {
    fn to_command(self) -> PathCommand {
        match self.0 % 4 {
            0 => PathCommand::Close,
            1 => PathCommand::MoveTo,
            2 => PathCommand::LineTo,
            _ => PathCommand::CubicTo,
        }
    }
}

#[derive(Arbitrary, Debug)]
struct Input {
    cmds: Vec<CmdIn>,
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
        let commands: Vec<PathCommand> = input.cmds.iter().map(|c| c.to_command()).collect();
        let points: Vec<Point> = input.pts.iter().map(|p| Point { x: p.x, y: p.y }).collect();
        // `commands` and `points` are independently sized so the
        // fuzzer routinely produces arity mismatches; the C engine
        // must reject those without overrunning the point buffer.
        let path = Path { commands, points };
        let _ = shape.append_path(&path);
    });
});
