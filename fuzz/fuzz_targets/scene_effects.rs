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
use thorvg::{
    BlurBorder, BlurDirection, DropShadow, GaussianBlur, Rect, Rgb, Rgba, Thorvg, Tint, Tritone,
};

#[derive(Arbitrary, Debug)]
enum Effect {
    GaussianBlur {
        sigma: f64,
        /// Mapped into [`BlurDirection`] via `% 3` so the enum's
        /// closed set still receives full coverage from the
        /// fuzzer's `u8` stream.
        direction: u8,
        /// Mapped into [`BlurBorder`] via `% 2`.
        border: u8,
        quality: u8,
    },
    DropShadow {
        r: u8,
        g: u8,
        b: u8,
        a: u8,
        angle: f64,
        distance: f64,
        sigma: f64,
        quality: u8,
    },
    Fill {
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    },
    Tint {
        r0: u8,
        g0: u8,
        b0: u8,
        r1: u8,
        g1: u8,
        b1: u8,
        intensity: f64,
    },
    Tritone {
        sr: u8,
        sg: u8,
        sb: u8,
        mr: u8,
        mg: u8,
        mb: u8,
        hr: u8,
        hg: u8,
        hb: u8,
        blend: u8,
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
                let _ = s.append_rect(Rect::new(c.x, c.y, c.w, c.h));
                let _ = s.set_fill_color(Rgba::new(255, 0, 0, 255));
                let _ = scene.add(s);
            }
        }
        for e in input.effects {
            let _ = match e {
                Effect::GaussianBlur {
                    sigma,
                    direction,
                    border,
                    quality,
                } => {
                    let direction = match direction % 3 {
                        0 => BlurDirection::Both,
                        1 => BlurDirection::Horizontal,
                        _ => BlurDirection::Vertical,
                    };
                    let border = if border % 2 == 0 {
                        BlurBorder::Duplicate
                    } else {
                        BlurBorder::Wrap
                    };
                    scene.add_gaussian_blur_effect(GaussianBlur {
                        sigma,
                        direction,
                        border,
                        quality,
                    })
                }
                Effect::DropShadow {
                    r,
                    g,
                    b,
                    a,
                    angle,
                    distance,
                    sigma,
                    quality,
                } => scene.add_drop_shadow_effect(DropShadow {
                    color: Rgba::new(r, g, b, a),
                    angle,
                    distance,
                    sigma,
                    quality,
                }),
                Effect::Fill { r, g, b, a } => scene.add_fill_effect(Rgba::new(r, g, b, a)),
                Effect::Tint {
                    r0,
                    g0,
                    b0,
                    r1,
                    g1,
                    b1,
                    intensity,
                } => scene.add_tint_effect(Tint {
                    black: Rgb::new(r0, g0, b0),
                    white: Rgb::new(r1, g1, b1),
                    intensity,
                }),
                Effect::Tritone {
                    sr,
                    sg,
                    sb,
                    mr,
                    mg,
                    mb,
                    hr,
                    hg,
                    hb,
                    blend,
                } => scene.add_tritone_effect(Tritone {
                    shadow: Rgb::new(sr, sg, sb),
                    midtone: Rgb::new(mr, mg, mb),
                    highlight: Rgb::new(hr, hg, hb),
                    blend,
                }),
                Effect::Clear => scene.clear_effects(),
            };
        }
    });
});
