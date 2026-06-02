use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let thorvg_src = manifest_dir.join("thorvg");

    if cfg!(feature = "vendored") {
        build_vendored_cc(&thorvg_src, &manifest_dir, &out_dir);
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
fn build_vendored_cc(thorvg_src: &Path, manifest_dir: &Path, out_dir: &Path) {
    let src = thorvg_src.join("src");

    // Write config.h based on enabled features.
    write_config_h(out_dir);

    // Canonical Cargo signals — see also the policy block below.  Read
    // here so the source-collection step can gate the bare-metal libc
    // shim TU on `target_os == "none"`.
    //
    // Three orthogonal predicates drive every target-derived decision:
    //
    //   * `is_bare_metal`  — `target_os == "none"`.  The system
    //                        provides nothing: no libc, no C++
    //                        runtime, no stdint.h.  Triggers the
    //                        full shim treatment.
    //   * `is_hosted`      — target_env is gnu/musl/msvc, or
    //                        target_vendor is apple.  The system
    //                        provides a working libc + libstdc++ and
    //                        cc-rs's defaults are correct.
    //   * everything else  — a self-contained runtime sits between
    //                        those two extremes: ESP-IDF, NuttX,
    //                        WASI, etc.  The runtime is provided
    //                        externally (SDK component, syscall ABI,
    //                        …); we just need to *not* emit
    //                        directives that conflict with what the
    //                        SDK will do at link time.
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    let is_bare_metal = target_os == "none";
    let is_msvc = target_env == "msvc";
    let is_hosted =
        matches!(target_env.as_str(), "gnu" | "musl" | "msvc") || target_vendor == "apple";
    // (`is_cross` is computed locally inside `generate_bindings`, which
    // is the only consumer.)

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

    // The vendored thorvg tree carries `src/common/tvgLibcShim.{h,cpp}`
    // from an earlier era when this crate replaced libc with an
    // in-tree weak-symbol shim.  Three paths converge on dropping
    // it from the source set:
    //
    //   * Hosted (`!is_bare_metal`):  the system libc already provides
    //     `strlen`/`strcmp`/etc. with strong linkage, so the weak
    //     shim symbols lose at link time — compiling them is just
    //     dead code in `libthorvg.a`.
    //   * Bare-metal with picolibc wired (`picolibc_active`):  picolibc
    //     provides every symbol the shim does, with stronger
    //     correctness (full UTF-8 ctype, locale-aware sorting that
    //     we leave disabled but is correctly stubbed, etc.).
    //   * Bare-metal *without* picolibc (legacy / non-wired arches):
    //     the shim stays, because nothing else provides those
    //     symbols.  Until arm / aarch64 / xtensa get their
    //     `libc/machine/<arch>/` enumeration in build.rs, this is
    //     the fallback that keeps those targets building.
    //
    // `picolibc_active` is computed below alongside the picolibc
    // build step; we defer the actual `sources.retain(...)` to after
    // that point so the gate has both signals available.

    // ── Include paths ────────────────────────────────────────────────────

    let mut include_dirs: Vec<PathBuf> = vec![
        out_dir.to_path_buf(),  // config.h
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

    // JerryScript's default global heap is 512 KB (`JERRY_GLOBAL_HEAP_SIZE`
    // in jerry-config.h:99), allocated as a single `.bss` static array.
    // That's larger than ESP32-C6's *entire* 512 KB SRAM, so on bare-metal
    // we cut it to a tight 16 KB — enough for the small expression
    // snippets typical Lottie files use, while leaving room for the
    // esp-alloc heap (416 KB), the ABGR backbuffer (230 KB at runtime),
    // and esp-hal's own .bss + stack.  Hosted builds keep the upstream
    // default.  At 32 KB the combined .bss overflows ESP32-C6's 512 KB
    // SRAM by ~14 KB; 16 KB just fits.
    if cfg!(feature = "expressions") && is_bare_metal {
        build.define("JERRY_GLOBAL_HEAP_SIZE", "16");
    }

    // Mirror the GCC/Clang flag set that upstream meson applies
    // unconditionally (`src/meson.build`).  Split into two tiers:
    //
    //   * The "size / correctness" subset is safe for every non-MSVC
    //     target — these are pure code-shape choices that match what
    //     thorvg upstream wants regardless of the link environment.
    //   * The unwind-stripping flags are bare-metal-only.  On SDK-style
    //     embedded targets (ESP-IDF, NuttX, …) the SDK's linker script
    //     asserts on the EH-frame section layout (e.g. ESP-IDF's
    //     `sections.ld` requires zero gap between `.flash.rodata` and
    //     `.eh_frame_hdr`).  Compiling our TUs with mismatched unwind
    //     policy vs. the SDK's own C++ TUs can leave the linker unable
    //     to satisfy those asserts.  On true `target_os == "none"` we
    //     control the link and want every byte stripped.
    if !is_msvc {
        for f in &[
            "-fno-exceptions",
            "-fno-rtti",
            "-fno-stack-protector",
            "-fno-math-errno",
        ] {
            build.flag_if_supported(f);
        }
    }
    if is_bare_metal {
        build.flag_if_supported("-fno-unwind-tables");
        build.flag_if_supported("-fno-asynchronous-unwind-tables");
    }

    // Non-hosted runtime plumbing.
    //
    // On any target that isn't a hosted Unix / Apple / Windows
    // platform, cc-rs's automatic `-lstdc++` / `-lc++` emission is
    // wrong: bare-metal has no system libstdc++, and SDK-style
    // runtimes (ESP-IDF, NuttX, WASI, …) bring their own as a
    // dedicated component and would conflict with a duplicate
    // emission here.  Suppress cc-rs's auto-link so each ecosystem
    // gets to handle C++ runtime linkage its own way.
    if !is_hosted {
        build.cpp_set_stdlib(None);
    }

    // Bare-metal-only additional plumbing.
    //
    // In a `target_os == "none"` environment the C++ runtime that gcc
    // normally injects is either missing or only partially present.
    // SDK-backed targets (ESP-IDF, etc.) handle these in their own
    // build glue and should not get them applied a second time here.
    //
    //   * `-fno-threadsafe-statics` suppresses the `__cxa_guard_*` calls
    //     emitted around function-local statics, which pull in pthread
    //     stubs unavailable on bare metal.
    //   * `-fno-use-cxa-atexit` avoids registrations against
    //     `__cxa_atexit`, which is a stub on bare-metal newlib.
    //   * The force-included libc shim header + dropping libc.a from
    //     the link replace newlib entirely.
    //   * `-Os` because code size dominates every reasonable
    //     bare-metal use of thorvg.
    if is_bare_metal {
        build.flag_if_supported("-fno-threadsafe-statics");
        build.flag_if_supported("-fno-use-cxa-atexit");
        build.opt_level_str("s");

        // The `-include tvgLibcShim.h` force-include used to live
        // here, force-pulling the in-tree weak ctype/str shim header
        // into every thorvg TU.  Replaced by picolibc's headers —
        // see the picolibc include-path block after the picolibc
        // compile call below, which conditionally adds `-nostdinc`
        // + picolibc's own header tree to thorvg's compile.
        //
        // RISC-V multilib normalisation.  cc-rs picks `-march=rv32imac`
        // (the short Rust-style form) for the riscv32imac-* triples;
        // most embedded toolchains (Espressif's riscv32-esp-elf, the
        // SiFive bsp, …) ship their no-FPU `libgcc.a` under the
        // *canonical* GCC multilib name `rv32imac_zicsr_zifencei` and
        // leave the toolchain's default multilib (`.`) built assuming
        // a hardware FPU.  Without this override, the
        // `cross_runtime_libs` probe returns the FPU-enabled libgcc
        // whose soft-float helpers (`__fixunsdfdi`, `__floatdidf`, …)
        // read `frm` / `fcsr` CSRs that don't exist on a pure rv32imac
        // chip, producing an illegal-instruction trap the first time
        // any `double`-to-integer conversion runs.
        //
        // Reconstruct the canonical march from
        // `CARGO_CFG_TARGET_FEATURE` (which Cargo populates from the
        // Rust target spec) and append `_zicsr_zifencei`, matching the
        // multilib naming used by every GCC ≥ 12 RISC-V cross
        // toolchain.  Applies to the cc compile *and*, via
        // `tool.args()`, to the runtime-libs / sysroot probes.
    }

    // Cross-toolchain multilib flag normalisation.
    //
    // This is NOT a picolibc concern — it's a cc-rs ↔ GCC flag-naming
    // mismatch that surfaces during the toolchain's runtime-archive
    // probes (`cross_runtime_libs`, `cross_sysroot_include`).  Currently
    // only RISC-V is affected:
    //
    //   cc-rs picks `-march=rv32imac` (the short Rust-style form) for the
    //   riscv32imac-* triples; most embedded toolchains (Espressif's
    //   riscv32-esp-elf, the SiFive bsp, …) ship their no-FPU `libgcc.a`
    //   under the *canonical* GCC multilib name `rv32imac_zicsr_zifencei`
    //   and leave the toolchain's default multilib (`.`) built assuming
    //   a hardware FPU.  Without this override, the `cross_runtime_libs`
    //   probe returns the FPU-enabled libgcc whose soft-float helpers
    //   (`__fixunsdfdi`, `__floatdidf`, …) read `frm` / `fcsr` CSRs that
    //   don't exist on a pure rv32imac chip, producing an
    //   illegal-instruction trap the first time any `double`-to-integer
    //   conversion runs.
    //
    // ARM / aarch64 cross toolchains agree with cc-rs on `-mcpu=` /
    // `-mfpu=` / `-mfloat-abi=` naming, so they need no equivalent
    // block — cc-rs's auto-emitted flags already select the right
    // multilib.  If a future arch shows the same mismatch, add it here
    // (and *not* inside `build_picolibc`, which stays arch-agnostic).
    let cross_toolchain_multilib_args: Vec<String> = if is_bare_metal && target_arch == "riscv32" {
        let features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
        let has = |c: char| {
            features
                .split(',')
                .any(|f| f.eq_ignore_ascii_case(&c.to_string()))
        };
        let mut isa = String::from("rv32i");
        for ext in ['m', 'a', 'f', 'd', 'c'] {
            if has(ext) {
                isa.push(ext);
            }
        }
        isa.push_str("_zicsr_zifencei");
        let abi = if has('d') {
            "ilp32d"
        } else if has('f') {
            "ilp32f"
        } else {
            "ilp32"
        };
        // We also include `-fno-rtti` here because the runtime-libs
        // probe needs to match cc-rs's compile (which gets it from the
        // upstream meson flag block above).  Without it, the probe
        // resolves to the `*/rtti/` multilib whose objects reference
        // typeinfo helpers our `-fno-rtti` build deliberately omits.
        vec![
            format!("-march={isa}"),
            format!("-mabi={abi}"),
            "-fno-rtti".to_string(),
        ]
    } else {
        Vec::new()
    };

    for f in &cross_toolchain_multilib_args {
        build.flag(f);
    }

    // -- Picolibc build (bare-metal) --------------------------------------
    //
    // On bare-metal targets where `build_picolibc` recognises the
    // architecture, we compile picolibc into a static archive
    // alongside thorvg and link the result.  `picolibc_active`
    // records the outcome so the rest of this function can pivot
    // header isolation and shim selection accordingly:
    //
    //   * Ok(()):    picolibc.a built and linked, thorvg's own
    //                compile switches to `-nostdinc` + picolibc
    //                headers, `tvgLibcShim.cpp` dropped from sources.
    //   * Err(why):  cargo:warning explains, and the legacy shim
    //                stays active so non-wired arches still build.
    //
    // Arch coverage is driven entirely by `picolibc_machine_subdir`:
    // any `target_arch` it maps to an existing `libc/machine/<dir>/`
    // is built; anything else returns Err and falls back to the shim.
    let picolibc_root = manifest_dir.join("picolibc");
    let picolibc_config = manifest_dir.join("picolibc-config");
    let picolibc_active = if is_bare_metal {
        match build_picolibc(
            &picolibc_root,
            &picolibc_config,
            &target_arch,
            &cross_toolchain_multilib_args,
        ) {
            Ok(()) => true,
            Err(reason) => {
                println!("cargo:warning=picolibc disabled, shim fallback active: {reason}");
                false
            }
        }
    } else {
        false
    };

    // -- Shim selection ---------------------------------------------------
    //
    // See the long comment up in the source-collection block: drop
    // `tvgLibcShim.cpp` whenever it isn't the active libc replacement.
    // The shim is only active when bare-metal AND picolibc isn't
    // wired for this arch.
    let shim_active = is_bare_metal && !picolibc_active;
    if !shim_active {
        sources.retain(|p| !p.to_string_lossy().contains("tvgLibcShim"));
    }

    // -- Thorvg header isolation under picolibc ---------------------------
    //
    // When picolibc is the active libc, thorvg's C++ TUs must see
    // picolibc's `<ctype.h>` / `<string.h>` / etc., not newlib's.
    // The same header-isolation policy `build_picolibc` applies to
    // picolibc's own sources — `-nostdinc` + explicit re-add of the
    // compiler builtin-includes — carries over here.
    //
    // Order matters: `picolibc-config/` first so `<picolibc.h>`
    // resolves to our hand-authored config; arch-specific machine
    // dir next; then `libc/stdio` + `libc/locale` (needed for
    // picolibc's internal cross-directory bare-name includes that
    // some of its public headers transitively pull in); then the
    // generic `libc/include/`.  Compiler builtins (`stdarg.h`,
    // `stddef.h`, `limits.h`) are restored via `-isystem`.
    if picolibc_active {
        // `-nostdinc` strips ALL of GCC/Clang's default include
        // search paths: libc headers (newlib), compiler builtins
        // (stdarg.h, stddef.h, …) AND libstdc++ headers.  We want
        // to drop only the libc set; everything else gets added back
        // explicitly below.
        build.flag("-nostdinc");

        // Picolibc tree: config first (resolves `<picolibc.h>`),
        // arch-specific machine dir next, then the internal
        // cross-directory bare-name dirs and finally the public
        // header tree.  See `build_picolibc` for the same ordering
        // applied to picolibc's own compile.
        build.include(&picolibc_config);
        let machine_subdir = picolibc_machine_subdir(&target_arch)
            .expect("picolibc_active gated on supported arches");
        build.include(picolibc_root.join("libc/machine").join(machine_subdir));
        build.include(picolibc_root.join("libc/stdio"));
        build.include(picolibc_root.join("libc/locale"));
        build.include(picolibc_root.join("libc/include"));

        // Restore compiler builtins (intrinsic headers) and the
        // C++ standard library (libstdc++ / libc++).  Both come
        // from the cross toolchain; picolibc replaces only libc.
        if let Some(builtin_inc) = cross_compiler_builtin_includes() {
            build.flag(format!("-isystem{}", builtin_inc.display()));
        }
        for cxx_inc in cross_cxx_include_paths() {
            build.flag(format!("-isystem{}", cxx_inc.display()));
        }
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
        let found = cross_runtime_libs(
            &[
                "libstdc++.a",
                "libc++.a",
                "libsupc++.a",
                "libgcc.a",
                "libm.a",
            ],
            &cross_toolchain_multilib_args,
        );
        let mut seen = std::collections::HashSet::new();
        for (dir, _) in &found {
            if seen.insert(dir.clone()) {
                println!("cargo:rustc-link-search=native={}", dir.display());
            }
        }
        for (_, name) in &found {
            println!("cargo:rustc-link-lib=static={name}");
        }
    } else if is_hosted {
        // System libc / libstdc++ — request dynamic linkage.  cc-rs
        // doesn't reliably emit this on cross builds, so we name the
        // libraries explicitly per platform.
        if target_vendor == "apple" || target_os == "freebsd" {
            println!("cargo:rustc-link-lib=dylib=c++");
        } else if target_os == "linux" || target_env == "gnu" {
            println!("cargo:rustc-link-lib=dylib=stdc++");
        }
    }
    // Non-hosted, non-bare-metal runtimes (ESP-IDF, NuttX, WASI, …)
    // emit nothing for the C++ runtime: the SDK is responsible for
    // putting libstdc++ / libgcc / libc on the link line.  cc-rs's
    // auto-emit was already suppressed above via
    // `cpp_set_stdlib(None)` so we don't need to fight it here.
    //
    // Note: linker-script-specific fixes (e.g. ESP-IDF's
    // `.eh_frame_hdr` layout assertion needing `-Wl,--no-eh-frame-hdr`)
    // do *not* belong here.  `cargo:rustc-link-arg` from a sys crate
    // applies only to that crate's own link products (its rlib has no
    // link step), so a directive emitted here would silently no-op
    // for the downstream binary that actually invokes the linker.
    // Such flags belong in the consumer's `.cargo/config.toml`
    // (`rustflags = ["-C", "link-arg=..."]`) or its own build.rs.

    println!("cargo:rerun-if-changed=thorvg/src");
}

// ---------------------------------------------------------------------------
// Vendored picolibc (bare-metal libc replacement)
// ---------------------------------------------------------------------------

/// Compile-only validation pass over the vendored picolibc tree.
///
/// Walks the curated set of C / .S source files under
/// `thorvg-sys/picolibc/libc/`, configures a fresh `cc::Build` with
/// our hand-authored `picolibc-config/picolibc.h` first on the
/// include path, and calls `cc::Build::try_compile("picolibc")`,
/// which produces a `libpicolibc.a` archive in OUT_DIR and emits
/// `cargo:rustc-link-search=native=<out>` plus
/// `cargo:rustc-link-lib=static=picolibc` so the resulting symbols
/// are visible to the final rustc link.
///
/// Returns `Ok(())` when the archive is built, or `Err(reason)`
/// when the architecture isn't yet wired or the compile failed.
/// The caller surfaces the reason via `cargo:warning=` and falls
/// back to the legacy `tvgLibcShim.cpp` source set — keeping the
/// build green on arches we haven't enumerated yet.
///
/// # Source enumeration
///
/// We walk the picolibc tree (rather than parsing its `meson.build`
/// files or hand-coding the full list) and apply two filters:
///
///   * A **path filter** — only files under directories we actually
///     want (`libc/ctype`, `libc/string`, `libc/stdlib`, …).
///   * A **denylist** — file basenames or basename suffixes we
///     never want in the build:
///       - `*_l.c`     locale-aware variants; we disabled locale
///                     in picolibc.h (`__MB_CAPABLE` undefined).
///       - `*_s.c`     C11 Annex K bounds-checking; not used.
///       - `wcs*.c`, `wmem*.c`, `wcp*.c`, `wcw*.c`, `wc[rs]*toc*.c`,
///         `mb[srl]*.c`, `mbtowc*.c`, `wctomb*.c`, `btowc.c`,
///         `wctob.c`  — wide-char and multi-byte machinery.
///       - `malloc.c`, `free.c`, `realloc.c`, `calloc.c`,
///         `aligned_alloc.c`, `memalign.c`, `posix-memalign.c`,
///         `valloc.c`, `pvalloc.c`, `reallocarray.c`, `reallocf.c`,
///         `mallinfo.c`, `mallopt.c`, `malloc-stats.c`,
///         `malloc-usable-size.c`  — consumer (esp-alloc, …)
///         provides the allocator.
///       - `getenv.c`, `getenv_r.c`, `putenv.c`, `setenv.c`,
///         `environ.c`, `system.c`  — no environment on bare metal.
///       - `drand48.c`, `erand48.c`, `jrand48.c`, `lrand48.c`,
///         `mrand48.c`, `nrand48.c`, `seed48.c`, `srand48.c`,
///         `lcong48.c`, `rand48.c`  — duplicate PRNGs; we keep
///         the plain `rand.c` / `srand.c` pair.
///       - `cxa-atexit.c`, `onexit.c`  — built with
///         `-fno-use-cxa-atexit`; we don't want the C++ side.
///       - `getopt.c`, `getsubopt.c`, `getauxval.c`,
///         `getpagesize.c`, `rpmatch.c`  — POSIX surface, unused.
///       - `assert.c`, `eprintf.c`  — assert paths pulling in stdio;
///         the `__ASSERT_VERBOSE` knob in picolibc.h covers what
///         we actually need via `assert_func.c`.
///       - `inittls.c`, `tls.c` (under `machine/`)  — TLS setup;
///         `__SINGLE_THREAD` deselects TLS for us.
///       - `lock.c`  — non-trivial lock helpers; collapsed to
///         no-ops by `<sys/lock.h>` under `__SINGLE_THREAD`.
///       - `picosbrk.c`, `init.c`, `fini.c`  — picocrt startup
///         glue; the consumer's HAL handles startup.
///       - The whole `libc/stdio/` directory is INCLUDED — option
///         (b) from the planning conversation (full tinystdio).
///
/// The denylist is intentionally explicit / file-level rather than
/// pattern-only: a future picolibc bump that adds a new
/// `strerror_l_new_variant.c` (or whatever) will land naturally
/// without needing build.rs changes, and a one-time `cargo build`
/// failure with a clear compiler error is a fine signal to update
/// the list.
///
/// # Architecture support
///
/// Coverage is driven by `picolibc_machine_subdir` (Rust `target_arch`
/// → picolibc `libc/machine/<dir>/`).  Any arch the helper maps to an
/// existing machine dir is built; arches it doesn't know about get a
/// structured `Err` and the caller logs a `cargo:warning=`.
///
/// The machine dir is walked the same way as the generic dirs — `.c`
/// and `.S` files at the top level (no recursion into nested
/// `machine/` header subdirs), filtered by `MACHINE_DENYLIST`.  This
/// keeps `build_picolibc` arch-agnostic: a new picolibc release that
/// adds files to an arch's machine dir is picked up automatically,
/// and adding a new arch is a one-line edit to
/// `picolibc_machine_subdir`.
///
/// ARM caveat: picolibc's `libc/machine/arm/` ships multiple ISA
/// variants of some files (e.g. `setjmp.S` per armv4t / armv6m /
/// armv7m / armv8m), gated by meson on `-mcpu=`.  A naive directory
/// walk would pick up all variants and produce duplicate-symbol link
/// errors.  When the first ARM consumer appears, port the meson
/// selection rule into a per-arch hook here.  Until then, ARM is
/// deliberately not in the translator table.
fn build_picolibc(
    picolibc_root: &Path,
    picolibc_config: &Path,
    target_arch: &str,
    cross_toolchain_multilib_args: &[String],
) -> Result<(), String> {
    // ── Arch resolution ───────────────────────────────────────────────

    let machine_subdir = picolibc_machine_subdir(target_arch).ok_or_else(|| {
        format!("target_arch={target_arch} not mapped to a picolibc machine dir")
    })?;

    let machine_dir = picolibc_root.join("libc/machine").join(machine_subdir);
    if !machine_dir.is_dir() {
        return Err(format!(
            "picolibc machine dir missing: {}",
            machine_dir.display()
        ));
    }

    // ── Generic sources (walked + denylisted) ─────────────────────────

    // Subtrees we want compiled, relative to `picolibc/`.
    let walk_dirs: &[&str] = &[
        "libc/ctype",
        "libc/string",
        "libc/stdlib",
        "libc/stdio",
        "libc/errno",
        "libc/search",
    ];

    let denylist_files = denylist_files();
    let denylist_suffixes: &[&str] = &["_l.c", "_s.c"];
    let denylist_prefixes: &[&str] = &[
        "wcs", "wmem", "wcp", "wcw", // wide-char
        "mblen", "mbr", "mbs", "mbt", "mbst", // multi-byte
    ];

    let mut sources: Vec<PathBuf> = Vec::new();
    for sub in walk_dirs {
        let dir = picolibc_root.join(sub);
        if !dir.is_dir() {
            return Err(format!("picolibc dir missing: {}", dir.display()));
        }
        for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let Some(ext) = path.extension() else {
                continue;
            };
            if ext != "c" {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if denylist_files.contains(&name) {
                continue;
            }
            if denylist_suffixes.iter().any(|s| name.ends_with(s)) {
                continue;
            }
            if denylist_prefixes.iter().any(|p| name.starts_with(p)) {
                continue;
            }
            sources.push(path);
        }
    }

    // ── Architecture-specific machine sources ─────────────────────────
    //
    // Walk the arch's machine dir (non-recursive — the nested
    // `machine/` subdir holds headers only, never sources) picking up
    // both `.c` and `.S` files.  Single shared denylist:
    // `tls.c` / `inittls.c` (TLS disabled by `__SINGLE_THREAD` in
    // picolibc.h).  Anything else picolibc ships per-arch — ieeefp
    // helpers, hand-written memcpy/memmove/strlen, setjmp.S, etc. —
    // comes along automatically, matching exactly what upstream
    // `libc/machine/<arch>/meson.build:srcs_machine` enumerates for
    // the arches whose machine dirs are flat (riscv, aarch64, x86,
    // x86_64, …).
    for entry in std::fs::read_dir(&machine_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if ext != "c" && ext != "S" {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if MACHINE_DENYLIST.contains(&name) {
            continue;
        }
        sources.push(path);
    }

    // ── thorvg-sys runtime stubs ──────────────────────────────────
    //
    // Strong-symbol implementations of the pthread no-op surface
    // libsupc++ pulls in unconditionally, plus `getenv` / `getentropy`
    // stubs.  Compiled into the picolibc archive so they share its
    // include-path / multilib configuration; conceptually they're
    // "libc surface picolibc leaves to the OS", not picolibc itself.
    let runtime_stubs = picolibc_config.join("runtime_stubs.c");
    if !runtime_stubs.is_file() {
        return Err(format!(
            "picolibc-config runtime_stubs.c missing: {}",
            runtime_stubs.display()
        ));
    }
    sources.push(runtime_stubs);

    // ── Include paths ─────────────────────────────────────────────────
    //
    // Order matters: our `picolibc-config/` *must* come first so that
    // every `#include <picolibc.h>` (both direct, from setjmp.S, and
    // indirect, via `<sys/cdefs.h>`) finds our authored config rather
    // than the upstream `.in` template (which isn't a valid C header).
    //
    // Then the per-arch `machine/` override (so e.g. `<machine/setjmp.h>`
    // picks up the riscv variant), then the generic include tree.

    // Mirror the include-dir set picolibc's meson build constructs
    // (`meson.build:inc_dirs`).  Beyond the obvious `libc/include`
    // (public headers) and `libc/machine/<arch>` (arch overrides),
    // picolibc TUs rely on `libc/stdio`, `libc/locale`, and
    // `libc/stdlib` being on the path so cross-directory bare-name
    // includes like `#include "locale_private.h"` from
    // `libc/ctype/local.h` resolve.  We don't *compile* anything from
    // `libc/locale/` — the locale knobs are off in `picolibc.h` —
    // but the headers still need to be reachable.  `libc/stdlib/` is
    // on the path so our `runtime_stubs.c` can pull picolibc's own
    // `local-onexit.h` for the `_on_exit` enum / union types
    // (keeps the stub signature welded to upstream — see
    // runtime_stubs.c).
    let include_dirs: Vec<PathBuf> = vec![
        picolibc_config.to_path_buf(),
        machine_dir.clone(),
        picolibc_root.join("libc/stdio"),
        picolibc_root.join("libc/locale"),
        picolibc_root.join("libc/stdlib"),
        picolibc_root.join("libc/include"),
    ];

    // ── cc::Build for picolibc ────────────────────────────────────────

    let mut build = cc::Build::new();
    build.warnings(false);

    // Plain C (not C++).  picolibc is all C99 sources.
    // `cpp(false)` is the default but we set it explicitly to make
    // intent obvious next to the existing C++ thorvg build above.
    build.cpp(false);

    // Drop the cross-toolchain's libc/system header search paths,
    // then add back the *compiler* builtin-header dir (stdarg.h,
    // stddef.h, limits.h, …) via `-isystem`.  This is the central
    // mechanism for header isolation in the picolibc landing: a
    // picolibc TU that does `#include <stdio.h>` *must* resolve to
    // picolibc's `libc/include/stdio.h`, never newlib's.
    build.flag("-nostdinc");
    if let Some(builtin_inc) = cross_compiler_builtin_includes() {
        build.flag(format!("-isystem{}", builtin_inc.display()));
    }

    // Match the size-tuned flag set thorvg's own build uses, so the
    // emitted .o files have consistent unwind / RTTI / stack-protector
    // policy.  (Most of these are no-ops for C TUs but cost nothing.)
    for f in &[
        "-fno-stack-protector",
        "-fno-math-errno",
        "-fno-unwind-tables",
        "-fno-asynchronous-unwind-tables",
    ] {
        build.flag_if_supported(f);
    }
    build.opt_level_str("s");

    // Mirror cc-rs's auto-suppression on bare-metal: we don't want
    // the C++ runtime auto-link here (picolibc is pure C and our
    // outer thorvg build already handles libstdc++ separately).
    build.cpp_set_stdlib(None);

    // Force-include `picolibc.h` so even TUs that don't include
    // `<sys/cdefs.h>` (e.g. some `.S` files routed through the C
    // preprocessor) see our config.
    let picolibc_h = picolibc_config.join("picolibc.h");
    build.flag(format!("-include{}", picolibc_h.display()));

    // Same multilib args as thorvg uses, so picolibc TUs build for
    // the same ABI as the rest of the link surface.  Empty (no flags
    // emitted) on arches where cc-rs and the cross toolchain agree
    // on multilib naming (ARM, aarch64, …) — see the long comment
    // next to `cross_toolchain_multilib_args` in `build_vendored_cc`.
    for f in cross_toolchain_multilib_args {
        build.flag(f);
    }

    for dir in &include_dirs {
        build.include(dir);
    }
    for src in &sources {
        build.file(src);
    }

    // Real link.  `try_compile("picolibc")` runs cc-rs's compile +
    // archive pipeline and emits `cargo:rustc-link-search=native=<out>`
    // + `cargo:rustc-link-lib=static=picolibc` so the archive is
    // visible to the final rustc link.  `try_*` variant chosen so a
    // failure surfaces as a structured `Err` (the caller logs and
    // falls back to the shim) rather than a `panic!` from `compile()`.
    build
        .try_compile("picolibc")
        .map_err(|e| format!("compile failed: {e}"))?;
    Ok(())
}

/// File-basenames excluded from the picolibc compile.
///
/// Kept as a function (not a `const`) because some entries are short
/// pattern groups (PRNG variants, malloc family) we want to read as
/// one logical block; the function form lets us comment each group
/// inline.  The whole list is filtered against `Path::file_name()`,
/// so subdirectory placement doesn't matter — a `malloc.c` anywhere
/// in the walked tree is dropped.
fn denylist_files() -> &'static [&'static str] {
    &[
        // Allocator — consumer (esp-alloc, embedded-alloc, …)
        // provides malloc/free/realloc/calloc as strong symbols.
        "malloc.c",
        "free.c",
        "realloc.c",
        "calloc.c",
        "aligned_alloc.c",
        "memalign.c",
        "posix-memalign.c",
        "valloc.c",
        "pvalloc.c",
        "reallocarray.c",
        "reallocf.c",
        "mallinfo.c",
        "mallopt.c",
        "malloc-stats.c",
        "malloc-usable-size.c",
        // Environment / POSIX — no environment on bare metal.
        "getenv.c",
        "getenv_r.c",
        "putenv.c",
        "setenv.c",
        "environ.c",
        "system.c",
        "getopt.c",
        "getsubopt.c",
        "getauxval.c",
        "getpagesize.c",
        "rpmatch.c",
        // 48-bit PRNG family (`drand48` / `erand48` / …) — same
        // generator with different output types; not used.
        "drand48.c",
        "erand48.c",
        "jrand48.c",
        "lrand48.c",
        "mrand48.c",
        "nrand48.c",
        "seed48.c",
        "srand48.c",
        "lcong48.c",
        "rand48.c",
        // Re-entrant rand variant — single-threaded build.
        "rand_r.c",
        // BSD base-64 ASCII encoders, unrelated to PRNG.
        "a64l.c",
        "l64a.c",
        // NOTE: `random.c` / `srandom.c` are KEPT (un-denylisted).
        // Earlier drafts dropped them under a "duplicate PRNG"
        // banner, but `random()` is a separate POSIX API from
        // `rand()` — different signature (`long int random(void)`
        // vs `int rand(void)`) and different output range.  The
        // cross-toolchain's libstdc++ — specifically the Lottie /
        // animation paths pulled by the `animation_basic` example
        // bin — takes a weak external reference to `random` that
        // the link surfaces if absent.  Cost is 133 lines of pure
        // C; gain is one less runtime_stubs.c entry.
        // `arc4random.c` pulls `getentropy` and a chacha-based
        // re-seeding loop we don't want.  Our `runtime_stubs.c`
        // provides a `getentropy` stub for libstdc++'s benefit,
        // and thorvg uses `rand()` (→ `rand.c`) when it needs a
        // sample for Lottie text-range randomisation.
        "arc4random.c",
        "arc4random_uniform.c",
        // C11 Annex K bounds-checking — not used.  (Covered by the
        // `_s.c` suffix denylist too, but `set_constraint_handler_s.c`
        // doesn't fit the pattern.)
        "set_constraint_handler_s.c",
        "ignore_handler_s.c",
        "strerrorlen_s.c",
        // C++ ABI atexit glue — we build with `-fno-use-cxa-atexit`.
        "cxa-atexit.c",
        "onexit.c",
        "exitprocs.c",
        // Multi-byte / wide-char single files not caught by prefix.
        "btowc.c",
        "wctob.c",
        "wctomb.c",
        "wctomb_r.c",
        "sb_charsets.c",
        "ejtouc.c",
        "jitouc.c",
        "sjtouc.c",
        "uctoej.c",
        "uctoji.c",
        "uctosj.c",
        // Wide-char ctype.
        "ctype_wide.c",
        // Assert family pulling stdio paths we don't need (the
        // verbose `__assert_func` from `assert_func.c` stays).
        "assert.c",
        "assert_no_arg.c",
        "eprintf.c",
        // POSIX wide-char console.
        "posixiob_stdin.c",
        "posixiob_stdout.c",
        "posixiob_stderr.c",
        // Stdio Ryu fast-but-large dtoa path — picolibc.h leaves
        // `__IO_FLOAT_EXACT` undefined, which selects the smaller
        // engine-based dtoa.  Drop the Ryu sources to keep object
        // count down.
        "atod_ryu.c",
        "atof_ryu.c",
        "dtoa_ryu.c",
        "ftoa_ryu.c",
        "ryu_divpow2.c",
        "ryu_log10.c",
        "ryu_log2pow5.c",
        "ryu_pow5bits.c",
        "ryu_table.c",
        "ryu_umul128.c",
        // Stdio templates — .c files that exist as preprocessor
        // sub-units (`#include`d from variant wrappers like
        // `vfprintf.c`, `vffprintf.c`, etc.) and are NOT compiled
        // standalone.  Picolibc upstream achieves this by simply
        // not listing them in `srcs_stdio`; our walk-the-dir
        // strategy needs them named explicitly.  Identifiable
        // because they reference `PRINTF_VARIANT` / `SCANF_VARIANT`
        // / `ULTOA_NAME` macros without defining them — those
        // come from the wrapper that pulls them in.
        "conv_flt.c",
        "ultoa_invert.c",
        "vfprintf_char.c",
        "vfprintf_float.c",
        "vfprintf_int.c",
        "vfprintf_n.c",
        "vfprintf_str.c",
        // Tree/hash search (libc/search) — we use bsearch + qsort only.
        "hash.c",
        "hash_bigkey.c",
        "hash_buf.c",
        "hash_func.c",
        "hash_log2.c",
        "hash_page.c",
        "hcreate.c",
        "hcreate_r.c",
        "ndbm.c",
        "tdelete.c",
        "tdestroy.c",
        "tfind.c",
        "tsearch.c",
        "twalk.c",
        "bsd_qsort_r.c",
        "qsort_r.c",
    ]
}

/// Files excluded from any `libc/machine/<arch>/` walk.
///
/// Single shared list across all arches because the exclusion
/// reasons are config-driven, not arch-driven:
///   * `tls.c` / `inittls.c` — TLS setup; `__SINGLE_THREAD` in
///     `picolibc.h` deselects TLS, so these would be dead code.
///
/// Kept tight on purpose.  If a future picolibc bump adds a
/// per-arch file we don't want, the right move is usually to add
/// the matching `__*` knob to `picolibc.h` (which removes the
/// reference path-wide) before reaching for this list.
const MACHINE_DENYLIST: &[&str] = &["tls.c", "inittls.c"];

/// Map a Rust `CARGO_CFG_TARGET_ARCH` value to picolibc's
/// `libc/machine/<dir>/` subdirectory name.
///
/// This is the single point of arch-policy in the picolibc build.
/// Every arch whose machine dir is a flat source set (no per-ISA
/// variant selection) lands here as a one-line entry and is built
/// automatically with no other code changes.
///
/// Returns `None` for arches picolibc upstream does not support
/// (xtensa, wasm32, …) and for arches that need per-ISA variant
/// selection that `build_picolibc`'s plain directory walk would
/// mis-resolve (arm — see the caveat in `build_picolibc`'s rustdoc).
/// Caller surfaces the `None` via `cargo:warning=` and falls back
/// to the legacy `tvgLibcShim.cpp` source set.
fn picolibc_machine_subdir(target_arch: &str) -> Option<&'static str> {
    // Matches the directory names that exist under
    // `picolibc/libc/machine/` in the vendored submodule.  When
    // bumping picolibc, sanity-check this table against
    // `ls picolibc/libc/machine/` — new arches in picolibc upstream
    // are additive, but the directory naming for x86 vs i386 etc.
    // has historically been stable.
    match target_arch {
        "riscv32" | "riscv64" => Some("riscv"),
        "aarch64" => Some("aarch64"),
        "x86" => Some("i386"),
        "x86_64" => Some("x86_64"),
        "powerpc" | "powerpc64" => Some("powerpc"),
        "mips" | "mips64" => Some("mips"),
        "sparc" | "sparc64" => Some("sparc"),
        "m68k" => Some("m68k"),
        "msp430" => Some("msp430"),
        // arm intentionally absent — its machine dir ships multiple
        // ISA-variant `.S` files (setjmp / memcpy / strcmp per
        // armv4t / armv6m / armv7m / armv8m) that picolibc's meson
        // selects between via `-mcpu=`.  A flat walk would pick all
        // variants and link-error on duplicate symbols.  Wire ARM
        // by porting that selection rule into `build_picolibc`.
        _ => None,
    }
}

/// Discover the cross-compiler's builtin-headers include directory.
///
/// GCC and Clang both ship their own copies of `<stdarg.h>`,
/// `<stddef.h>`, `<limits.h>`, `<stdint.h>`, `<float.h>` etc., which
/// are pulled in by picolibc TUs even though picolibc itself doesn't
/// ship these (they're compiler-builtin, not libc).
///
/// `-nostdinc` (used by `build_picolibc`) suppresses *all* implicit
/// system header paths, including these builtins.  We restore only
/// the builtins by probing the cross compiler with
/// `-print-file-name=include` (GCC) and falling back to
/// `-print-resource-dir` + `/include` (Clang).
///
/// Returns `None` when the driver produces no useful answer; the
/// caller falls back to building without builtin headers, which
/// will fail loudly at the first `#include <stddef.h>` and surface
/// a clear error.
fn cross_compiler_builtin_includes() -> Option<PathBuf> {
    let tool = cc::Build::new().try_get_compiler().ok()?;

    // GCC path: `-print-file-name=include` returns the absolute path
    // to the toolchain-bundled include dir.
    let mut cmd = std::process::Command::new(tool.path());
    cmd.args(tool.args());
    cmd.arg("-print-file-name=include");
    if let Ok(res) = cmd.output() {
        let s = String::from_utf8_lossy(&res.stdout).trim().to_string();
        if !s.is_empty() && s != "include" {
            let p = PathBuf::from(s);
            if p.is_dir() {
                return Some(p);
            }
        }
    }

    // Clang path: `-print-resource-dir` returns `<resource>`; the
    // include dir is `<resource>/include`.
    let mut cmd = std::process::Command::new(tool.path());
    cmd.args(tool.args());
    cmd.arg("-print-resource-dir");
    if let Ok(res) = cmd.output() {
        let s = String::from_utf8_lossy(&res.stdout).trim().to_string();
        if !s.is_empty() {
            let p = PathBuf::from(s).join("include");
            if p.is_dir() {
                return Some(p);
            }
        }
    }

    None
}

/// Discover the cross-compiler's C++ stdlib include search paths.
///
/// `-nostdinc` (applied to thorvg's C++ compile when picolibc is
/// active) strips ALL default include paths — libc, compiler
/// builtins, and libstdc++ all disappear together.  Picolibc
/// replaces libc and `cross_compiler_builtin_includes()` restores
/// the compiler intrinsics, but libstdc++ still needs to come back
/// for thorvg's C++ TUs (`<string>`, `<vector>`, `<algorithm>`, …).
///
/// GCC and Clang both expose their full default-include search list
/// via the `-E -x c++ -v` probe: the `-v` output contains a block
/// labelled `#include <...> search starts here:` listing every
/// directory `-nostdinc` would otherwise add by default.  We parse
/// that block and keep only the paths whose absolute name contains
/// `/c++/` — the universal marker for libstdc++ / libc++ trees
/// (`include/c++/<gcc-version>`, `include/c++/<gcc-version>/<triple>`,
/// `include/c++/<gcc-version>/backward` on GCC;
/// `include/c++/v1` on Clang's libc++).  Compiler builtins and
/// libc paths fall through and never reach our `-isystem` flags;
/// the builtin probe handles the former and we deliberately drop
/// the latter.
///
/// Returns an empty vector when the probe fails or the toolchain
/// has no C++ support — in which case the thorvg compile will
/// surface a clear `<string>: No such file or directory` error,
/// which is the right signal: this code path is C++-only.
fn cross_cxx_include_paths() -> Vec<PathBuf> {
    let Ok(tool) = cc::Build::new().cpp(true).try_get_compiler() else {
        return Vec::new();
    };
    let mut cmd = std::process::Command::new(tool.path());
    cmd.args(tool.args());
    // `-E -x c++ -v -` makes the driver:
    //   * run preprocessor-only (`-E`),
    //   * in C++ mode (`-x c++`),
    //   * print its include search paths (`-v`),
    //   * read the source from stdin (`-`).
    // Combined with `Stdio::null()` for stdin, that produces an
    // empty C++ translation unit whose only purpose is to make the
    // driver emit its standard search-path diagnostic.
    cmd.arg("-E")
        .arg("-x")
        .arg("c++")
        .arg("-v")
        .arg("-")
        .stdin(std::process::Stdio::null());
    let Ok(res) = cmd.output() else {
        return Vec::new();
    };
    // The `-v` diagnostic goes to stderr.
    let stderr = String::from_utf8_lossy(&res.stderr);

    let mut paths = Vec::new();
    let mut in_block = false;
    for line in stderr.lines() {
        if line.contains("#include <...> search starts here:") {
            in_block = true;
            continue;
        }
        if line.contains("End of search list.") {
            break;
        }
        if !in_block {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Universal marker for C++ stdlib trees.  Conservative on
        // purpose: libc paths sometimes live under the same prefix
        // as the cross sysroot but they never contain `/c++/`.
        if !trimmed.contains("/c++/") {
            continue;
        }
        let p = PathBuf::from(trimmed);
        if p.is_dir() {
            paths.push(p);
        }
    }
    paths
}

// ---------------------------------------------------------------------------
// Cross-toolchain runtime-archive discovery
// ---------------------------------------------------------------------------

/// Discover bare-metal runtime archives from the configured cross-compiler.
///
/// For each `libfoo.a` in `wanted`, asks the cross C++ compiler for
/// its multilib-correct location via `-print-file-name=libfoo.a`.
/// Returns `(directory, link_name)` pairs in the order the archives
/// were probed — the caller emits `rustc-link-search=` once per
/// distinct directory and `rustc-link-lib=static=<link_name>` for
/// each entry.  Archives the driver can't find are silently skipped
/// (some toolchains fold libsupc++ into libstdc++, for example).
fn cross_runtime_libs(wanted: &[&str], extra_args: &[String]) -> Vec<(PathBuf, String)> {
    let Ok(tool) = cc::Build::new().cpp(true).try_get_compiler() else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(wanted.len());
    for file in wanted {
        let mut cmd = std::process::Command::new(tool.path());
        // Forward cc-rs's resolved compile args + any caller-supplied
        // multilib selectors (e.g. RISC-V canonical
        // `-march=rv32imac_zicsr_zifencei` / `-mabi=ilp32` /
        // `-fno-rtti`).  Without them, embedded GCC toolchains return
        // the *default* multilib, which on most RISC-V cross builds
        // was built assuming a hardware FPU.  Linking its soft-float
        // helpers (`__fixunsdfdi`, `__floatdidf`, …) into a no-FPU
        // chip (`rv32imac` / `ilp32`) produces SIGILL the first time
        // any `double`-to-integer conversion runs at runtime.
        cmd.args(tool.args());
        for f in extra_args {
            cmd.arg(f);
        }
        cmd.arg(format!("-print-file-name={file}"));
        let Ok(res) = cmd.output() else { continue };
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

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let host = env::var("HOST").unwrap_or_default();
    let target = env::var("TARGET").unwrap_or_default();
    let is_cross = !target.is_empty() && target != host;

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

    // On cross-compilation targets bindgen invokes libclang directly,
    // *not* the cross-compiler.  libclang's default include search list
    // is the host's, so headers like `<stdint.h>` from the cross
    // toolchain's sysroot (newlib on bare metal, ESP-IDF's libcxx on
    // espidf, …) are invisible — every `uint8_t` / `uint32_t` in
    // `thorvg_capi.h` then fails to resolve.  Ask the user-configured
    // cross-compiler where its sysroot lives and feed its `include/`
    // dir to libclang, and set `--target=` so the ABI matches.
    //
    // We key off `TARGET != HOST` rather than `target_os == "none"`
    // because the same fix is needed for any cross target with a
    // non-host sysroot (ESP-IDF, NuttX, embedded Linux, …).
    if is_cross {
        if let Some(inc) = cross_sysroot_include() {
            builder = builder.clang_arg(format!("-I{}", inc.display()));
        }
        // libclang doesn't recognise vendor-specific OS fields like
        // `espidf` in Rust triples, and the ABI fields it does parse
        // (`unknown`, `eabihf`, …) aren't load-bearing for header
        // parsing.  Strip both: `<arch>-none-elf` is the LLVM triple
        // libclang understands across every embedded target we care
        // about, and the arch is the only field that affects
        // sizeof/alignof for `uint32_t` etc.
        builder = builder.clang_arg(format!("--target={target_arch}-none-elf"));
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
    let mut cmd = std::process::Command::new(tool.path());
    // Forward `-march` / `-mabi` etc. so that toolchains which keep a
    // per-multilib sysroot return the right include tree (currently
    // newlib's `<sysroot>/include` is shared across multilibs, but
    // matching the runtime-libs probe avoids subtle drift later).
    cmd.args(tool.args());
    cmd.arg("-print-sysroot");
    let out = cmd.output().ok()?;
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
