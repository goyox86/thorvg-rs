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
/// Cross-compiler selection follows cc-rs's standard contract: the user
/// supplies `CC_<triple>` / `CXX_<triple>` (or `TARGET_CC` / `TARGET_CXX`)
/// in their environment or `.cargo/config.toml`.  This crate does not
/// hardcode any vendor-specific binary names.
///
/// Which loaders and capabilities are compiled is controlled by cargo
/// features.  Target-derived policy is limited to the canonical
/// `CARGO_CFG_TARGET_*` signals — chiefly `target_os == "none"` for
/// bare-metal — matching the convention used by `ring`, `mbedtls-sys-auto`,
/// and other no_std sys crates.
fn build_vendored_cc(thorvg_src: &Path, out_dir: &Path) {
    let src = thorvg_src.join("src");

    // Write config.h based on enabled features.
    write_config_h(out_dir);

    // Canonical Cargo signals — see also the policy block below.  Read
    // here so the source-collection step can gate the bare-metal libc
    // shim TU on `target_os == "none"`.
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let is_bare_metal = target_os == "none";
    let is_msvc = target_env == "msvc";

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

    // The bare-metal libc shim (`tvgLibcShim.cpp`) lives under
    // `src/common/`, so the directory glob above picked it up
    // unconditionally.  On hosted targets the system libc already
    // provides `strlen`/`strcmp`/etc. with strong linkage, so the
    // weak shim symbols would lose at link time anyway — but
    // compiling and shipping dead code in `libthorvg.a` is wasteful.
    // Strip the shim from the source set whenever we're not bare metal.
    if !is_bare_metal {
        sources.retain(|p| !p.to_string_lossy().contains("tvgLibcShim"));
    }

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
    //
    // `target_os == "none"` is Rust's canonical bare-metal indicator
    // (`*-unknown-none-elf`, `*-none-eabi*`, …) and is what `ring`,
    // `mbedtls-sys-auto`, and the rest of the no_std sys-crate ecosystem
    // key off.  Cross-compiler selection is left entirely to the user
    // via `CC_<triple>` / `CXX_<triple>` in `.cargo/config.toml`.
    // (`target_os` / `target_env` / `is_bare_metal` / `is_msvc` were
    // already computed above the source collection step.)

    // Expose POSIX functions (strdup, strncasecmp, etc.) on newlib
    // toolchains where they are gated behind feature-test macros.
    build.define("_DEFAULT_SOURCE", None);

    // JerryScript's JERRY_VLA fallback uses alloca() without including
    // <alloca.h>.  Override the macro to use C99 VLAs which GCC and
    // Clang support natively.
    if cfg!(feature = "expressions") {
        build.define("JERRY_VLA(type,name,size)", "type name[size]");
    }

    // Mirror the GCC/Clang flag set that upstream meson applies
    // unconditionally (`src/meson.build`).  These shrink code size and
    // — critically for bare-metal targets — remove the EH/unwind
    // metadata that would otherwise reference libstdc++ symbols absent
    // from an embedded sysroot.
    if !is_msvc {
        for f in &[
            "-fno-exceptions",
            "-fno-rtti",
            "-fno-stack-protector",
            "-fno-math-errno",
            "-fno-unwind-tables",
            "-fno-asynchronous-unwind-tables",
        ] {
            build.flag_if_supported(f);
        }
    }

    // Bare-metal additional plumbing.
    //
    // In a `target_os == "none"` environment the C++ runtime that gcc
    // normally injects is either missing or only partially present:
    //
    //   * `-fno-threadsafe-statics` suppresses the `__cxa_guard_*` calls
    //     emitted around function-local statics, which pull in pthread
    //     stubs unavailable on bare metal.
    //   * `-fno-use-cxa-atexit` avoids registrations against
    //     `__cxa_atexit`, which is a stub on bare-metal newlib.
    //   * `cpp_set_stdlib(None)` stops cc-rs from emitting `-lstdc++` /
    //     `-lc++`: the cross-compiler driver already pulls the correct
    //     libstdc++ multilib from its own sysroot when invoked as the
    //     linker.
    //
    // Optimise for size on embedded targets too — code size dominates
    // every reasonable bare-metal use of thorvg.
    if is_bare_metal {
        build.flag_if_supported("-fno-threadsafe-statics");
        build.flag_if_supported("-fno-use-cxa-atexit");
        build.cpp_set_stdlib(None);
        build.opt_level_str("s");

        // Force-include the in-tree libc shim header so every TU sees
        // ASCII inlines for `<ctype.h>` + `atoi` / `strtol` before any
        // system header has a chance to declare them.  Pairs with the
        // weak-linkage `str*` shims compiled from `tvgLibcShim.cpp`,
        // and lets us drop `libc.a` from the link entirely (avoiding
        // ODR collisions with HAL-provided symbols such as
        // `__stack_chk_fail`).
        let shim = src.join("common").join("tvgLibcShim.h");
        build.flag(format!("-include{}", shim.display()));
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

    // Link the C++ runtime (and friends on bare metal).
    //
    // Hosted Unix platforms get a dynamic `c++` / `stdc++` request.
    // On bare metal (`target_os == "none"`) the Rust project usually
    // links with `rust-lld`, which won't search the cross toolchain
    // for libstdc++/libgcc/libc on its own.  Ask the configured
    // cross-compiler where each archive lives (`-print-file-name=`),
    // and emit a `rustc-link-search` + `rustc-link-lib=static=` for
    // every one we find.  This stays toolchain-agnostic: any GCC- or
    // Clang-based cross toolchain pointed at by `CXX_<triple>` works.
    //
    // The archives we look for:
    //   * `libstdc++.a` / `libc++.a` — the C++ standard library;
    //     thorvg uses `std::string`, `<algorithm>`, etc.
    //   * `libsupc++.a` — C++ ABI / runtime helpers (`operator new`,
    //     EH personality).  Some toolchains fold this into libstdc++.
    //   * `libgcc.a` — compiler-rt equivalents (soft-float, divide,
    //     `_Unwind_*` referenced by the EH personality even when the
    //     consumer code is built `-fno-exceptions`).
    //   * `libc.a` — newlib's standard C library (`isspace`, `memcpy`,
    //     `malloc` shims, …).
    if is_bare_metal {
        // `libc.a` is intentionally *not* probed: thorvg's ctype /
        // str-family / parser needs are covered by `tvgLibcShim.{h,cpp}`
        // (force-included header + weak-linkage TU compiled above), and
        // pulling newlib's libc.a tends to drag objects like
        // `stack_protector.o` that collide with HAL-defined symbols
        // (`__stack_chk_fail` on esp-hal, for example).  `libm.a` is
        // required because thorvg uses sqrt/sin/cos/atan2 extensively
        // and replacing those is impractical; libm.a does not ship
        // colliding shims on any toolchain we know of.
        let found = cross_runtime_libs(&[
            "libstdc++.a",
            "libc++.a",
            "libsupc++.a",
            "libgcc.a",
            "libm.a",
        ]);
        let mut seen = std::collections::HashSet::new();
        for (dir, _) in &found {
            if seen.insert(dir.clone()) {
                println!("cargo:rustc-link-search=native={}", dir.display());
            }
        }
        for (_, name) in &found {
            println!("cargo:rustc-link-lib=static={name}");
        }
    } else {
        let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();
        if target_vendor == "apple" || target_os == "freebsd" {
            println!("cargo:rustc-link-lib=dylib=c++");
        } else if target_os == "linux" || target_env == "gnu" {
            println!("cargo:rustc-link-lib=dylib=stdc++");
        }
    }

    println!("cargo:rerun-if-changed=thorvg/src");
}

/// Discover bare-metal runtime archives from the configured cross-compiler.
///
/// For each `libfoo.a` in `wanted`, asks the cross C++ compiler for
/// its multilib-correct location via `-print-file-name=libfoo.a`.
/// Returns `(directory, link_name)` pairs in the order the archives
/// were probed — the caller emits `rustc-link-search=` once per
/// distinct directory and `rustc-link-lib=static=<link_name>` for
/// each entry.  Archives the driver can't find are silently skipped
/// (some toolchains fold libsupc++ into libstdc++, for example).
fn cross_runtime_libs(wanted: &[&str]) -> Vec<(PathBuf, String)> {
    let Ok(tool) = cc::Build::new().cpp(true).try_get_compiler() else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(wanted.len());
    for file in wanted {
        let res = std::process::Command::new(tool.path())
            .arg(format!("-print-file-name={file}"))
            .output();
        let Ok(res) = res else { continue };
        let s = String::from_utf8_lossy(&res.stdout).trim().to_string();
        // Drivers echo the bare filename when they can't locate the lib.
        if s == *file {
            continue;
        }
        let p = PathBuf::from(s);
        if !p.is_file() {
            continue;
        }
        let Some(dir) = p.parent() else { continue };
        let link = file
            .trim_start_matches("lib")
            .trim_end_matches(".a")
            .to_string();
        out.push((dir.to_path_buf(), link));
    }
    out
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

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let is_bare_metal = target_os == "none";

    let mut builder = bindgen::Builder::default()
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
        .layout_tests(true);

    // On bare-metal targets bindgen invokes libclang directly, *not*
    // the cross-compiler.  libclang's default include search list is
    // the host's, so headers like `<stdint.h>` from the embedded
    // newlib sysroot are invisible — every `uint8_t` / `uint32_t` in
    // `thorvg_capi.h` then fails to resolve.  Ask the user-configured
    // cross-compiler where its sysroot lives and feed its `include/`
    // dir to libclang, and set `--target=` so the ABI matches.
    if is_bare_metal {
        if let Some(inc) = cross_sysroot_include() {
            builder = builder.clang_arg(format!("-I{}", inc.display()));
        }
        // The Rust triple's vendor/ABI fields (`unknown`, `eabihf`, …)
        // aren't load-bearing for header parsing, but the arch is:
        // pass a plain LLVM triple that libclang recognises.
        builder = builder.clang_arg(format!("--target={}-none-elf", target_arch));
    }

    let bindings = builder.generate().expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

/// Discover the cross-compiler's sysroot include directory.
///
/// Asks the user-configured C++ compiler (resolved through cc-rs's
/// standard `CXX_<triple>` / `TARGET_CXX` / `CXX` precedence) for its
/// `-print-sysroot`, then probes `<sysroot>/include`.  Used to point
/// libclang at the cross toolchain's newlib headers when generating
/// bindings for a bare-metal target.
fn cross_sysroot_include() -> Option<PathBuf> {
    let tool = cc::Build::new().cpp(true).try_get_compiler().ok()?;
    let out = std::process::Command::new(tool.path())
        .arg("-print-sysroot")
        .output()
        .ok()?;
    let sysroot = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if sysroot.is_empty() {
        return None;
    }
    let inc = PathBuf::from(sysroot).join("include");
    inc.exists().then_some(inc)
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
