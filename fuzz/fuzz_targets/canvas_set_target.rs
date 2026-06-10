#![no_main]

//! Fuzz target for `SwCanvas::set_target` and the canvas mutation
//! surface (`push` / `update` / `draw` / `sync` / `set_viewport` /
//! `render`).
//!
//! Specifically targets the `stride * height` overflow check in the
//! wrapper: the obvious `u32 * u32` would wrap in release builds and
//! let pathological sizes pass the bound check, after which thorvg
//! would compute the same product on its side and walk past the
//! buffer.  Inputs combine arbitrary `(stride, width, height,
//! colorspace, buffer_len)` with arbitrary viewport rectangles.

use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use thorvg::{ColorSpace, Rgba, Thorvg};

#[derive(Debug)]
enum FuzzCs {
    Abgr8888,
    Argb8888,
    Abgr8888s,
    Argb8888s,
}

impl<'a> Arbitrary<'a> for FuzzCs {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(match u.int_in_range::<u8>(0..=3)? {
            0 => Self::Abgr8888,
            1 => Self::Argb8888,
            2 => Self::Abgr8888s,
            _ => Self::Argb8888s,
        })
    }
}

impl FuzzCs {
    fn to_cs(&self) -> ColorSpace {
        match self {
            Self::Abgr8888 => ColorSpace::ABGR8888,
            Self::Argb8888 => ColorSpace::ARGB8888,
            Self::Abgr8888s => ColorSpace::ABGR8888S,
            Self::Argb8888s => ColorSpace::ARGB8888S,
        }
    }
}

#[derive(Debug)]
struct Input {
    /// Reported stride (may differ from `width` — wrapper should
    /// validate `stride * height` against `buffer.len()`).
    stride: u32,
    width: u32,
    height: u32,
    /// Backing buffer size in u32s — independent of the reported
    /// dimensions so the overflow check has something to bite on.
    buffer_len: u32,
    cs: FuzzCs,
    viewport: (i32, i32, i32, i32),
    /// Number of dummy shapes to push before draw.
    children: u8,
}

impl<'a> Arbitrary<'a> for Input {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            // Skew dimensions so the overflow path gets hit on a
            // meaningful fraction of inputs: half the time pick a
            // value near `u32::MAX`, otherwise pick a small one.
            stride: pick_stride(u)?,
            width: u.int_in_range(0..=4096)?,
            height: pick_stride(u)?,
            // Keep the actual allocation bounded so libfuzzer doesn't
            // OOM — the wrapper's bound check must reject huge
            // requested sizes regardless of allocation budget.
            buffer_len: u.int_in_range(0..=64 * 1024)?,
            cs: FuzzCs::arbitrary(u)?,
            viewport: (
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
            ),
            children: u.int_in_range(0..=8)?,
        })
    }
}

fn pick_stride(u: &mut Unstructured<'_>) -> arbitrary::Result<u32> {
    if u.arbitrary::<bool>()? {
        Ok(u.int_in_range(0..=4096)?)
    } else {
        Ok(u.int_in_range(u32::MAX / 2..=u32::MAX)?)
    }
}

thread_local! {
    static ENGINE: Thorvg = Thorvg::init(0).expect("thorvg init");
}

fuzz_target!(|input: Input| {
    ENGINE.with(|engine| {
        let Ok(mut canvas) = engine.sw_canvas(Default::default()) else {
            return;
        };
        let mut buffer = vec![0u32; input.buffer_len as usize];
        // SAFETY: buffer outlives the canvas borrow; the wrapper's
        // own bound check is what we're stressing.
        let r = unsafe {
            canvas.set_target(
                &mut buffer,
                input.stride,
                input.width,
                input.height,
                input.cs.to_cs(),
            )
        };
        if r.is_err() {
            return;
        }
        for _ in 0..input.children {
            if let Ok(mut s) = engine.shape() {
                let _ = s.append_rect(0.0, 0.0, 8.0, 8.0, 0.0, 0.0, true);
                let _ = s.set_fill_color(Rgba::new(255, 0, 0, 255));
                let _ = canvas.add(s);
            }
        }
        let (vx, vy, vw, vh) = input.viewport;
        let _ = canvas.set_viewport(vx, vy, vw, vh);
        let _ = canvas.update();
        let _ = canvas.draw(true);
        let _ = canvas.sync();
        let _ = canvas.clear();
    });
});
