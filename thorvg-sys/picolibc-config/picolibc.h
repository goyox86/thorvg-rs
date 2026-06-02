/*
 * picolibc.h — hand-authored for thorvg-sys bare-metal builds.
 *
 * Upstream picolibc generates this header from `picolibc.h.in` via
 * meson at configure time.  We don't run meson; instead we ship
 * this static replacement on the include path *above* the vendored
 * picolibc tree (which doesn't carry a `picolibc.h` of its own —
 * meson is what materialises one in `<builddir>/`).
 *
 * Every `#cmakedefine` in the upstream template is resolved here to
 * a static `#define` (set) or absence (unset), with the choices
 * tuned for thorvg's use on `target_os == "none"` targets:
 *
 *   * Single-threaded.  Bare-metal binaries do not run pthread.
 *     `__SINGLE_THREAD` collapses every lock to a no-op (see
 *     `libc/include/sys/lock.h`).
 *   * Plain global `errno`.  No TLS, no `__errno_location()` —
 *     `__GLOBAL_ERRNO` selects the same shape our prior
 *     `tvgLibcShim.cpp` already had (a single int per binary).
 *   * IEEE math (no `errno` from `<math.h>` wrappers).  thorvg's
 *     renderer takes the IEEE result and never reads `errno` from
 *     a math call.
 *   * Full Ryu-based stdio (`__TINY_STDIO` + Ryu printers).  We
 *     vendor picolibc's tinystdio so consumers downstream get
 *     accurate `%f` / `%g` for any thorvg output that hits stdio
 *     (Lottie expression debug, SVG number serialisation, …).
 *     The `__NANO_FORMATTED_IO` slimmed variant is rejected — its
 *     accuracy gap shows up in font advance widths and gradient
 *     stops.
 *   * No locale, no multibyte charsets, no semihosting, no MMU,
 *     no posix-console.  None are used by thorvg, and each disabled
 *     knob removes another transitive header / TU dependency from
 *     the build.
 *   * `_LITE_EXIT` + static atexit table.  Bare-metal `main()`
 *     never returns, so the heavy atexit machinery is dead code.
 *
 * Anything missing here that picolibc TUs reference at preprocess
 * time will fail compilation loudly — that's deliberate, the build
 * surfaces a `#error` line rather than silently choosing a different
 * runtime shape.  When a knob needs flipping, edit it here, not in
 * build.rs (which intentionally has no `-D__*` flags for these).
 */

#pragma once

/* ── Threading / locking ─────────────────────────────────────────── */

/* Collapses sys/lock.h to no-ops; suppresses __libc_lock acquire/
 * release calls inside tinystdio.  Required: we have no pthread. */
#define __SINGLE_THREAD

/* TLS storage selectors — all off.  Picolibc falls back to a single
 * global errno + a global atexit table when none of these are set. */
/* #undef __THREAD_LOCAL_STORAGE */
/* #undef __THREAD_LOCAL_STORAGE_API */
/* #undef __THREAD_LOCAL_STORAGE_STACK_GUARD */

/* Plain `int errno;` (vs. `int* __errno()` indirection).  Matches the
 * shape `tvgLibcShim.cpp`'s `__errno` weak-symbol stub provided. */
#define __GLOBAL_ERRNO

/* When __GLOBAL_ERRNO is set, `__PICOLIBC_ERRNO_FUNCTION` is unused;
 * leave the symbol undefined to make accidental references compile-
 * fail rather than silently expand to nothing. */
/* #undef __PICOLIBC_ERRNO_FUNCTION */

/* atexit table is a single shared array (single-threaded).  Without
 * `_ATEXIT_DYNAMIC_ALLOC`, the table is a fixed-size .bss array —
 * fine for bare-metal where atexit gets at most a handful of entries
 * from C++ static destructors. */
#define _GLOBAL_ATEXIT
/* #undef _ATEXIT_DYNAMIC_ALLOC */

/* ── Math ────────────────────────────────────────────────────────── */

/* `<math.h>` wrappers do not set `errno`.  thorvg uses sqrt/sin/cos/
 * atan2 freely and never reads errno from any of them. */
#define __IEEE_LIBM
/* #undef __MATH_ERRNO */

/* IEEEFP funcs (isnanl, finite, etc. — POSIX/IEEE 754 funcs distinct
 * from the C standard math library) — off; thorvg doesn't use them. */
/* #undef __IEEEFP_FUNCS */

/* These two only matter for picolibc's libm build, which we do NOT
 * compile from picolibc — the cross-toolchain's libm.a still provides
 * sqrt etc.  Set to 0 so the header parses and the value never gets
 * consulted. */
#define __OBSOLETE_MATH_FLOAT  0
#define __OBSOLETE_MATH_DOUBLE 0

/* ── Stdio ───────────────────────────────────────────────────────── */

/* picolibc's modern stdio implementation.  Provides FILE*, fopen,
 * vsnprintf, fprintf, …  Replaces the minimal vsnprintf/snprintf
 * the old `tvgLibcShim.cpp` carried. */
#define __TINY_STDIO

/* Format width.  'd' = double-precision floats supported; 'f' = float
 * only; 'l' = long double; 'i' = integer-only; 'm' = minimal.  thorvg
 * formats f32/f64 (Lottie expression eval, SVG attr emission) so we
 * need 'd'. */
#define __IO_DEFAULT 'd'

/* C99 printf flags (`hh`, `ll`, `j`, `z`, `t`).  thorvg uses `%zu`
 * for sizes. */
#define __IO_C99_FORMATS

/* `%lld` etc. supported.  Cheap, kept on. */
#define __IO_LONG_LONG

/* Use the full-precision (`__IO_FLOAT_EXACT` = off + Ryu) path.  Ryu
 * matches glibc/musl output to the last bit without the table-driven
 * `__IO_FLOAT_EXACT` blowup.  Slimming to `__NANO_FORMATTED_IO` shaves
 * a few KB but produces visibly-wrong fractional outputs for the
 * gradient stops / SVG numbers thorvg emits. */
/* #undef __IO_FLOAT_EXACT */
/* #undef __NANO_FORMATTED_IO */
/* #undef __IO_MINIMAL_LONG_LONG */

/* Security: `%n` is the format-string-attack vector and we never need
 * it.  Same for `%b` (binary printf, a picolibc extension). */
/* #undef __IO_PERCENT_N */
/* #undef __IO_PERCENT_B */

/* Other stdio knobs — all off for size. */
/* #undef __IO_POS_ARGS */
/* #undef __IO_WCHAR */
/* #undef __IO_LONG_DOUBLE */
/* #undef __IO_SMALL_ULTOA */
/* #undef __WIDE_ORIENT */

/* fseek micro-optimisation (avoids buffer flush when seeking inside
 * the buffer).  Pure win when fseek is used, zero cost otherwise. */
#define __FSEEK_OPTIMIZATION

/* `fvwrite_in_streamio` — the heavy multi-iovec write path.  Not
 * needed for the `printf`-style call sites thorvg has. */
/* #undef __FVWRITE_IN_STREAMIO */

/* Atomic ungetc — only matters when multiple threads share a FILE*.
 * `__SINGLE_THREAD` already covers it; keep off. */
/* #undef __ATOMIC_UNGETC */

/* Unbuffered stream optimisation — keep off; we don't use unbuffered
 * mode. */
/* #undef __UNBUF_STREAM_OPT */

/* ── Memory allocator ────────────────────────────────────────────── */

/* Consumer (esp-alloc on esp-hal, embedded-alloc on Cortex-M, …)
 * provides malloc/free/realloc/calloc as strong C symbols.  We
 * do NOT enable picolibc's nano-malloc or default malloc TUs. */
/* #undef __NANO_MALLOC */
/* #undef __MALLOC_SMALL_BUCKET */

/* ── Locale / multibyte / wide char ──────────────────────────────── */

/* All off.  thorvg's text rendering is byte-oriented (UTF-8 input,
 * SFNT glyph indices) and never touches mbstowcs/wctomb. */
/* #undef __MB_CAPABLE */
/* #undef __MB_EXTENDED_CHARSETS_ALL */
/* #undef __MB_EXTENDED_CHARSETS_UCS */
/* #undef __MB_EXTENDED_CHARSETS_ISO */
/* #undef __MB_EXTENDED_CHARSETS_WINDOWS */
/* #undef __MB_EXTENDED_CHARSETS_JIS */

/* ── Init / exit ─────────────────────────────────────────────────── */

/* GCC/clang emit `.init_array` / `.fini_array` for static ctor/dtor
 * registration on every target we care about (riscv32/64, arm,
 * aarch64, xtensa). */
#define __INIT_FINI_ARRAY
/* #undef __INIT_FINI_FUNCS */

/* `_exit()` is a tiny stub on bare metal (the consumer's HAL provides
 * it, or it traps to a debug breakpoint).  `_LITE_EXIT` skips the
 * heavy `__call_exitprocs` traversal. */
#define _LITE_EXIT

/* `register_fini` — picocrt-side helper.  We don't build picocrt. */
/* #undef _WANT_REGISTER_FINI */

/* ── OS surface ──────────────────────────────────────────────────── */

/* Bare metal has no fcntl, no posix console, no semihosting routed
 * through picolibc, and no MMU init.  HALs handle their own console
 * and trap dispatch. */
/* #undef __HAVE_FCNTL */
/* #undef POSIX_CONSOLE */
/* #undef __SEMIHOST */
/* #undef __PICOCRT_ENABLE_MMU */
/* #undef __PICOCRT_RUNTIME_SIZE */

/* ── Compiler capabilities ───────────────────────────────────────── *
 *
 * Every supported toolchain (GCC ≥ 4.8, clang ≥ 6) provides the
 * features below.  Set them statically — meson would otherwise probe
 * via tiny compile tests we can't run from build.rs without
 * complicating the script. */

/* GCC `__attribute__((alias))` — used inside picolibc for printf
 * variant aliases (`__d_vfprintf` → `vfprintf` etc.). */
#define __strong_reference(sym, aliassym) \
    extern __typeof(sym) aliassym __attribute__((__alias__(#sym)))

/* `__attribute__((packed))` honours bitfield layout — true for all
 * our toolchains. */
#define __HAVE_BITFIELDS_IN_PACKED_STRUCTS

/* `_Complex` type — supported.  Picolibc gates `<complex.h>` on it. */
#define __HAVE_COMPLEX

/* `-fno-builtin` / `-fno-tree-loop-distribute-patterns` to prevent
 * GCC from spotting `for(i;i<n;i++) dst[i]=src[i]` and rewriting it
 * to a memcpy call (which would link back to the very memcpy we're
 * trying to define).  cc-rs doesn't pass these by default; if a
 * downstream user complains about a runaway memcpy self-call,
 * setting this is the fix.  Off by default — the call sites in
 * picolibc are already hand-written asm or have inhibiting
 * attributes upstream. */
/* #undef __HAVE_CC_INHIBIT_LOOP_TO_LIBCALL */

/* ── Size vs. speed ──────────────────────────────────────────────── */

#define __PREFER_SIZE_OVER_SPEED
/* #undef __FAST_STRCMP */

/* ── Diagnostic verbosity ────────────────────────────────────────── */

/* assert() prints file:line:func.  Without this, it just calls
 * abort() — fine for production, painful for diagnosis.  Keep on. */
#define __ASSERT_VERBOSE

/* ── Newlib heritage version (printed in `<sys/features.h>`) ─────── *
 *
 * Picolibc 1.8.x reports its newlib base as 4.3.0 (see
 * `meson.build:NEWLIB_VERSION` in the vendored tree). */
#define _NEWLIB_VERSION       "4.3.0"
#define __NEWLIB__            4
#define __NEWLIB_MINOR__      3
#define __NEWLIB_PATCHLEVEL__ 0

/* ── Picolibc version ────────────────────────────────────────────── *
 *
 * Pinned in the submodule to tag `1.8.11`.  Bump both this block and
 * the submodule pointer together when upgrading. */
#define _PICOLIBC_VERSION       "1.8.11"
#define _PICOLIBC__             1
#define _PICOLIBC_MINOR__       8
#define __PICOLIBC_VERSION__    "1.8.11"
#define __PICOLIBC__            1
#define __PICOLIBC_MINOR__      8
#define __PICOLIBC_PATCHLEVEL__ 11

/* ── Xtensa-specific ─────────────────────────────────────────────── *
 *
 * Espressif's xtensa cross toolchain ships `xtensa/config/core-isa.h`
 * via its sysroot.  When building for an xtensa target with that
 * toolchain on the include path, define this so picolibc's
 * `libc/include/machine/xtensa/...` headers pick it up instead of
 * falling back to a generic stub.  Off by default — riscv / arm /
 * aarch64 builds never see this knob. */
/* #undef _XTENSA_HAVE_CONFIG_CORE_ISA_H */
