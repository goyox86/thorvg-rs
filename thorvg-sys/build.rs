use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let thorvg_src = manifest_dir.join("thorvg");

    if cfg!(feature = "vendored") {
        build_vendored_cc(&thorvg_src, &out_dir);
    } else {
        link_system();
    }

    generate_bindings(&thorvg_src, &out_dir);
}

// ---------------------------------------------------------------------------
// cc-based vendored build
// ---------------------------------------------------------------------------

/// Build ThorVG from vendored source using the `cc` crate.
///
/// The `cc` crate automatically picks up the correct cross-compiler from
/// Cargo's target environment (e.g. `xtensa-esp32s3-elf-g++` for ESP32-S3,
/// `arm-none-eabi-g++` for Cortex-M, host `g++`/`clang++` for desktop).
///
/// Which loaders and capabilities are compiled is controlled entirely by
/// cargo features — no target-based heuristics.
fn build_vendored_cc(thorvg_src: &Path, out_dir: &Path) {
    let src = thorvg_src.join("src");
    let target = env::var("TARGET").unwrap_or_default();

    // Write config.h based on enabled features.
    write_config_h(out_dir);

    // ── Collect source files ─────────────────────────────────────────────

    let mut sources: Vec<PathBuf> = Vec::new();

    // Core (always compiled): renderer, SW engine, common, C API, raw loader.
    add_dir_cpp(&mut sources, &src.join("renderer"));
    add_dir_cpp(&mut sources, &src.join("renderer/cpu_engine"));
    add_dir_cpp(&mut sources, &src.join("common"));
    add_dir_cpp(&mut sources, &src.join("bindings/capi"));
    add_dir_cpp(&mut sources, &src.join("loaders/raw"));

    // Feature-gated loaders.
    if cfg!(feature = "lottie") {
        add_dir_cpp(&mut sources, &src.join("loaders/lottie"));

        if cfg!(feature = "expressions") {
            add_dir_cpp_recursive(&mut sources, &src.join("loaders/lottie/jerryscript"));
        } else {
            // Without expressions, exclude the expressions source file
            // (it references JerryScript headers).
            sources.retain(|p| !p.to_string_lossy().contains("tvgLottieExpressions"));
        }
    }

    if cfg!(feature = "svg") {
        add_dir_cpp(&mut sources, &src.join("loaders/svg"));
    }

    if cfg!(feature = "png") {
        add_dir_cpp(&mut sources, &src.join("loaders/png"));
    }

    if cfg!(feature = "fonts") {
        add_dir_cpp(&mut sources, &src.join("loaders/sfnt"));
    }

    // Always exclude GPU engine files.
    sources.retain(|p| {
        let s = p.to_string_lossy();
        !s.contains("gpu_engine") && !s.contains("tvgGl") && !s.contains("tvgWg")
    });

    // ── Include paths ────────────────────────────────────────────────────

    let mut include_dirs: Vec<PathBuf> = vec![
        out_dir.to_path_buf(), // config.h
        thorvg_src.join("inc"), // thorvg.h public C++ header
        src.join("renderer"),
        src.join("renderer/cpu_engine"),
        src.join("common"),
        src.join("bindings/capi"),
        src.join("loaders/raw"),
    ];

    if cfg!(feature = "lottie") {
        include_dirs.push(src.join("loaders/lottie"));
        include_dirs.push(src.join("loaders/lottie/rapidjson"));

        if cfg!(feature = "expressions") {
            for sub in &[
                "jerry-core",
                "jerry-core/include",
                "jerry-core/ecma/base",
                "jerry-core/ecma/builtin-objects",
                "jerry-core/ecma/builtin-objects/typedarray",
                "jerry-core/ecma/operations",
                "jerry-core/jcontext",
                "jerry-core/jmem",
                "jerry-core/jrt",
                "jerry-core/lit",
                "jerry-core/parser/js",
                "jerry-core/parser/regexp",
                "jerry-core/vm",
                "jerry-port/common",
            ] {
                include_dirs.push(src.join("loaders/lottie/jerryscript").join(sub));
            }
        }
    }

    if cfg!(feature = "svg") {
        include_dirs.push(src.join("loaders/svg"));
    }

    if cfg!(feature = "png") {
        include_dirs.push(src.join("loaders/png"));
    }

    if cfg!(feature = "fonts") {
        include_dirs.push(src.join("loaders/sfnt"));
    }

    // ── Build ────────────────────────────────────────────────────────────

    let mut build = cc::Build::new();
    build.cpp(true).std("c++14").warnings(false);

    // -- Target-specific plumbing (not policy) ----------------------------

    // Expose POSIX functions (strdup, strncasecmp, etc.) on newlib
    // toolchains where they are gated behind feature-test macros.
    build.define("_DEFAULT_SOURCE", None);

    // JerryScript's JERRY_VLA fallback uses alloca() without including
    // <alloca.h>.  Override the macro to use C99 VLAs which GCC and
    // Clang support natively.
    if cfg!(feature = "expressions") {
        build.define("JERRY_VLA(type,name,size)", "type name[size]");
    }

    // ESP-IDF provides its own C++ runtime (libcxx component) — prevent
    // the cc crate from auto-linking libstdc++ which would cause
    // multiple-definition errors.
    if target.contains("espidf") {
        build.cpp_set_stdlib(None);
    }

    // Xtensa target selection — the generic `xtensa-esp-elf-g++` defaults
    // to big-endian.  The target-specific wrappers select the correct
    // multilib (little-endian) automatically.
    if target.contains("esp32s3") {
        build.compiler("xtensa-esp32s3-elf-g++");
    } else if target.contains("esp32s2") {
        build.compiler("xtensa-esp32s2-elf-g++");
    } else if target.contains("esp32") && target.contains("xtensa") {
        build.compiler("xtensa-esp32-elf-g++");
    }

    // Optimise for size on small targets.
    if target.contains("xtensa")
        || target.contains("riscv32")
        || target.contains("thumbv")
        || target.contains("arm")
    {
        build.opt_level_str("s");
    }

    // -- Compile -----------------------------------------------------------

    for dir in &include_dirs {
        build.include(dir);
    }
    for src_file in &sources {
        build.file(src_file);
    }

    build.compile("thorvg");

    println!("cargo:rustc-link-lib=static=thorvg");

    // Link C++ runtime where needed.
    if target.contains("espidf") {
        // ESP-IDF links its own C++ support; nothing to add.
    } else if target.contains("apple") || target.contains("freebsd") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else if target.contains("linux") || target.contains("gnu") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
    // Other bare-metal targets link C++ via their sysroot automatically.

    println!("cargo:rerun-if-changed=thorvg/src");
}

/// Write `config.h` based on enabled cargo features.
fn write_config_h(out_dir: &Path) {
    let mut config = String::from(
        "/* Generated by thorvg-sys build.rs — do not edit. */\n\
         #pragma once\n\n\
         #define THORVG_CAPI_BINDING_SUPPORT 1\n\
         #define THORVG_CPU_ENGINE_SUPPORT 1\n\
         #define THORVG_VERSION_STRING \"1.0.5\"\n",
    );

    if cfg!(feature = "lottie") {
        config.push_str("#define THORVG_LOTTIE_LOADER_SUPPORT 1\n");
    }
    if cfg!(feature = "expressions") {
        config.push_str("#define THORVG_LOTTIE_EXPRESSIONS_SUPPORT 1\n");
    }
    if cfg!(feature = "svg") {
        config.push_str("#define THORVG_SVG_LOADER_SUPPORT 1\n");
    }
    if cfg!(feature = "png") {
        config.push_str("#define THORVG_PNG_LOADER_SUPPORT 1\n");
    }
    if cfg!(feature = "fonts") {
        config.push_str("#define THORVG_SFNT_LOADER_SUPPORT 1\n");
        config.push_str("#define THORVG_OTF_LOADER_SUPPORT 1\n");
        config.push_str("#define THORVG_TTF_LOADER_SUPPORT 1\n");
    }
    if cfg!(feature = "threads") {
        config.push_str("#define THORVG_THREAD_SUPPORT 1\n");
    }
    if cfg!(feature = "file-io") {
        config.push_str("#define THORVG_FILE_IO_SUPPORT 1\n");
    }

    std::fs::write(out_dir.join("config.h"), config).expect("failed to write config.h");
}

// ---------------------------------------------------------------------------
// System (non-vendored) build
// ---------------------------------------------------------------------------

/// Link against a system-installed thorvg via pkg-config.
fn link_system() {
    pkg_config::Config::new()
        .atleast_version("1.0.0")
        .probe("thorvg")
        .expect(
            "Could not find system thorvg >= 1.0.0 via pkg-config. \
             Either install thorvg or enable the `vendored` feature.",
        );
}

// ---------------------------------------------------------------------------
// Bindgen
// ---------------------------------------------------------------------------

/// Generate Rust bindings from the C API header.
fn generate_bindings(thorvg_src: &Path, out_dir: &Path) {
    let capi_header = thorvg_src
        .join("src")
        .join("bindings")
        .join("capi")
        .join("thorvg_capi.h");

    println!("cargo:rerun-if-changed={}", capi_header.display());

    let bindings = bindgen::Builder::default()
        .header(capi_header.to_string_lossy())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .clang_arg(format!("-I{}", out_dir.display()))
        .allowlist_function("tvg_.*")
        .allowlist_type("Tvg_.*")
        .allowlist_var("TVG_.*")
        .rustified_enum("Tvg_Result")
        .rustified_enum("Tvg_Colorspace")
        .rustified_enum("Tvg_Engine_Option")
        .rustified_enum("Tvg_Mask_Method")
        .rustified_enum("Tvg_Blend_Method")
        .rustified_enum("Tvg_Type")
        .rustified_enum("Tvg_Stroke_Cap")
        .rustified_enum("Tvg_Stroke_Join")
        .rustified_enum("Tvg_Stroke_Fill")
        .rustified_enum("Tvg_Fill_Rule")
        .rustified_enum("Tvg_Text_Wrap")
        .rustified_enum("Tvg_Filter_Method")
        .use_core()
        .layout_tests(true)
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Collect all `.cpp` files in a directory (non-recursive).
fn add_dir_cpp(out: &mut Vec<PathBuf>, dir: &Path) {
    if !dir.exists() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "cpp") {
                out.push(path);
            }
        }
    }
}

/// Collect all `.cpp` files in a directory tree (recursive).
fn add_dir_cpp_recursive(out: &mut Vec<PathBuf>, dir: &Path) {
    if !dir.exists() {
        return;
    }
    for entry in walkdir(dir) {
        if entry.extension().is_some_and(|e| e == "cpp") {
            out.push(entry);
        }
    }
}

/// Simple recursive directory walk (avoids pulling in the `walkdir` crate).
fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                result.extend(walkdir(&path));
            } else {
                result.push(path);
            }
        }
    }
    result
}
