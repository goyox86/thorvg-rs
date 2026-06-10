#![no_main]

//! Fuzz target for `Scene` effect chains.
//!
//! Builds a scene populated with N arbitrary shapes, then applies
//! an arbitrary sequence of post-processing effects:
//! `add_gaussian_blur_effect`, `add_drop_shadow_effect`, `add_fill_effect`,
//! `add_tint_effect`, `add_tritone_effect`, `clear_effects`.  All
//! effect parameters are arbitrary `i32`/`f64`s; the targets here
//! are: (1) the wrapper's enum/bound translation, and (2) any
//! out-of-range handling the C engine assumes about RGBA bytes,
//! quality, angle, sigma, etc.

use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use thorvg::Thorvg;

#[derive(Arbitrary, Debug)]
enum Effect {
    GaussianBlur {
        sigma: f64,
        direction: i32,
        border: i32,
        quality: i32,
    },
    DropShadow {
        r: i32, g: i32, b: i32, a: i32,
        angle: f64, distance: f64, sigma: f64, quality: i32,
    },
    Fill {
        r: i32, g: i32, b: i32, a: i32,
    },
    Tint {
        r0: i32, g0: i32, b0: i32,
        r1: i32, g1: i32, b1: i32,
        intensity: f64,
    },
    Tritone {
        sr: i32, sg: i32, sb: i32,
        mr: i32, mg: i32, mb: i32,
        hr: i32, hg: i32, hb: i32,
        blend: i32,
    },
    Clear,
}

#[derive(Arbitrary, Debug)]
struct ChildRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Debug)]
struct Input {
    children: Vec<ChildRect>,
    effects: Vec<Effect>,
}

impl<'a> Arbitrary<'a> for Input {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let nc = u.int_in_range::<u8>(0..=8)? as usize;
        let mut children = Vec::with_capacity(nc);
        for _ in 0..nc {
            children.push(ChildRect::arbitrary(u)?);
        }
        let ne = u.int_in_range::<u8>(0..=16)? as usize;
        let mut effects = Vec::with_capacity(ne);
        for _ in 0..ne {
            effects.push(Effect::arbitrary(u)?);
        }
        Ok(Self { children, effects })
    }
}

thread_local! {
    static ENGINE: Thorvg = Thorvg::init(0).expect("thorvg init");
}

fuzz_target!(|input: Input| {
    ENGINE.with(|engine| {
        let Ok(mut scene) = engine.scene() else {
            return;
        };
        for c in input.children {
            if let Ok(mut s) = engine.shape() {
                let _ = s.append_rect(c.x, c.y, c.w, c.h, 0.0, 0.0, true);
                let _ = s.set_fill_color(255, 0, 0, 255);
                let _ = scene.push(s);
            }
        }
        for e in input.effects {
            let _ = match e {
                Effect::GaussianBlur { sigma, direction, border, quality } => {
                    scene.add_gaussian_blur_effect(sigma, direction, border, quality)
                }
                Effect::DropShadow { r, g, b, a, angle, distance, sigma, quality } => {
                    scene.add_drop_shadow_effect(r, g, b, a, angle, distance, sigma, quality)
                }
                Effect::Fill { r, g, b, a } => scene.add_fill_effect(r, g, b, a),
                Effect::Tint { r0, g0, b0, r1, g1, b1, intensity } => {
                    scene.add_tint_effect(r0, g0, b0, r1, g1, b1, intensity)
                }
                Effect::Tritone { sr, sg, sb, mr, mg, mb, hr, hg, hb, blend } => {
                    scene.add_tritone_effect(sr, sg, sb, mr, mg, mb, hr, hg, hb, blend)
                }
                Effect::Clear => scene.clear_effects(),
            };
        }
    });
});
