#![no_main]

//! Fuzz target for `LinearGradient` / `RadialGradient` construction
//! and attachment.
//!
//! Builds a gradient with arbitrary color-stop arrays (the wrapper
//! casts `len` to `u32`), arbitrary bounds / radial parameters,
//! spread mode, and transform matrix.  Optionally attaches it to a
//! shape via `set_linear_gradient`, `set_radial_gradient`, or
//! `set_stroke_linear_gradient` / `set_stroke_radial_gradient` to
//! also exercise the ownership-transfer
//! path.  Read-side getters are queried at the end to catch any
//! mis-deserialisation of stops/bounds/transform.

use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use thorvg::{ColorStop, FillSpread, LinearGradient, Matrix, RadialGradient, Rect, Thorvg};

#[derive(Arbitrary, Debug)]
struct ArbStop {
    offset: f32,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[derive(Arbitrary, Debug)]
enum Kind {
    Linear {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
    Radial {
        cx: f32,
        cy: f32,
        r: f32,
        fx: f32,
        fy: f32,
        fr: f32,
    },
}

#[derive(Arbitrary, Debug)]
enum Attach {
    Linear,
    Radial,
    StrokeLinear,
    StrokeRadial,
    None,
}

#[derive(Debug)]
struct Input {
    kind: Kind,
    stops: Vec<ArbStop>,
    spread: u8,
    transform: [f32; 9],
    attach: Attach,
}

impl<'a> Arbitrary<'a> for Input {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let n = u.int_in_range::<u8>(0..=64)? as usize;
        let mut stops = Vec::with_capacity(n);
        for _ in 0..n {
            stops.push(ArbStop::arbitrary(u)?);
        }
        Ok(Self {
            kind: Kind::arbitrary(u)?,
            stops,
            spread: u.arbitrary()?,
            transform: u.arbitrary()?,
            attach: Attach::arbitrary(u)?,
        })
    }
}

fn to_spread(b: u8) -> FillSpread {
    match b % 3 {
        0 => FillSpread::Pad,
        1 => FillSpread::Reflect,
        _ => FillSpread::Repeat,
    }
}

fn to_matrix(m: [f32; 9]) -> Matrix {
    Matrix {
        e11: m[0],
        e12: m[1],
        e13: m[2],
        e21: m[3],
        e22: m[4],
        e23: m[5],
        e31: m[6],
        e32: m[7],
        e33: m[8],
    }
}

fn stops_vec(arb: &[ArbStop]) -> Vec<ColorStop> {
    arb.iter()
        .map(|s| ColorStop {
            offset: s.offset,
            r: s.r,
            g: s.g,
            b: s.b,
            a: s.a,
        })
        .collect()
}

thread_local! {
    static ENGINE: Thorvg = Thorvg::init(0).expect("thorvg init");
}

fn run_linear<'a>(engine: &'a Thorvg, input: &Input) -> Option<LinearGradient<'a>> {
    let mut g = engine.linear_gradient().ok()?;
    if let Kind::Linear { x1, y1, x2, y2 } = input.kind {
        let _ = g.set_bounds(x1, y1, x2, y2);
    }
    let _ = g.set_color_stops(&stops_vec(&input.stops));
    let _ = g.set_spread(to_spread(input.spread));
    let _ = g.set_transform(&to_matrix(input.transform));
    let _ = g.bounds();
    let _ = g.color_stops();
    let _ = g.spread();
    let _ = g.get_transform();
    let _ = g.gradient_type();
    Some(g)
}

fn run_radial<'a>(engine: &'a Thorvg, input: &Input) -> Option<RadialGradient<'a>> {
    let mut g = engine.radial_gradient().ok()?;
    if let Kind::Radial {
        cx,
        cy,
        r,
        fx,
        fy,
        fr,
    } = input.kind
    {
        let _ = g.set_radial(cx, cy, r, fx, fy, fr);
    }
    let _ = g.set_color_stops(&stops_vec(&input.stops));
    let _ = g.set_spread(to_spread(input.spread));
    let _ = g.set_transform(&to_matrix(input.transform));
    let _ = g.radial();
    let _ = g.color_stops();
    let _ = g.spread();
    let _ = g.get_transform();
    let _ = g.gradient_type();
    Some(g)
}

fuzz_target!(|input: Input| {
    ENGINE.with(|engine| {
        // Always build the kind matching `input.kind`; for the
        // non-matching getter we still want full coverage so build
        // both flavours when stops are non-empty.
        let lin = run_linear(engine, &input);
        let rad = run_radial(engine, &input);

        // Attach exactly one of them to a freshly-built shape.
        let Ok(mut shape) = engine.shape() else {
            return;
        };
        let _ = shape.append_rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        match input.attach {
            Attach::Linear => {
                if let Some(g) = lin {
                    let _ = shape.set_linear_gradient(g);
                }
            }
            Attach::Radial => {
                if let Some(g) = rad {
                    let _ = shape.set_radial_gradient(g);
                }
            }
            Attach::StrokeLinear => {
                if let Some(g) = lin {
                    let _ = shape.set_stroke_linear_gradient(g);
                }
            }
            Attach::StrokeRadial => {
                if let Some(g) = rad {
                    let _ = shape.set_stroke_radial_gradient(g);
                }
            }
            Attach::None => {}
        }
        let _ = shape.gradient();
        let _ = shape.stroke_gradient();
    });
});
