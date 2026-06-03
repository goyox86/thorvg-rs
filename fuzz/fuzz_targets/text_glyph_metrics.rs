#![no_main]

//! Fuzz target for the font-loading + text-shaping path.
//!
//! Registers an arbitrary byte buffer as a font in the engine-wide
//! registry, attaches it to a `Text` paint, sets arbitrary UTF-8
//! content, and queries `text_metrics` / `glyph_metrics`.  Font
//! parsers are a classic source of OOB reads; this target drives
//! the whole pipeline that lives behind `Thorvg::load_font_data`,
//! `Text::set_font`, `Text::set_text`, and the metric getters.

use libfuzzer_sys::arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use thorvg::Thorvg;

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    font_bytes: &'a [u8],
    mime: Option<&'a str>,
    text: &'a str,
    glyph: &'a str,
    size: f32,
    italic: f32,
    letter: f32,
    line: f32,
}

const FONT_NAME: &str = "fuzz-font";

thread_local! {
    static ENGINE: Thorvg = Thorvg::init(0).expect("thorvg init");
}

fuzz_target!(|input: Input<'_>| {
    ENGINE.with(|engine| {
        // Registration may fail (e.g. embedded NULs in `mime`); we
        // still want to exercise the rest of the path on failure.
        let _ = engine.load_font_data(FONT_NAME, input.font_bytes, input.mime);
        let Ok(mut text) = engine.text() else {
            let _ = engine.unload_font_from_str(FONT_NAME);
            return;
        };
        let _ = text.set_font(FONT_NAME);
        let _ = text.set_size(input.size);
        let _ = text.set_italic(input.italic);
        let _ = text.set_spacing(input.letter, input.line);
        let _ = text.set_text(input.text);
        let _ = text.text_metrics();
        let _ = text.glyph_metrics(input.glyph);
        let _ = text.line_count();
        drop(text);
        // Keep the global font registry from growing across iterations.
        let _ = engine.unload_font_from_str(FONT_NAME);
    });
});
