/*
 * picolibc.h — hand-authored compile-time configuration.
 *
 * Upstream picolibc generates this from `picolibc.h.in` via meson.
 * We don't run meson; instead this static replacement sits *first*
 * on the include path so every picolibc and thorvg TU sees the same
 * configuration.  Each `#cmakedefine` from the upstream template is
 * resolved here to a `#define` (set) or absence (unset).
 *
 * Any knob picolibc TUs reference at preprocess time but not covered
 * here will fail compilation loudly — intended.  Edit knobs here,
 * not via `-D__*` flags in build.rs.
 */

#pragma once

/* ── Threading / locking ─────────────────────────────────────────── */

/* Collapses sys/lock.h to no-ops and suppresses tinystdio's
 * `__libc_lock` calls.  Required: bare-metal has no pthread. */
#define __SINGLE_THREAD

/* TLS off — picolibc falls back to a single global errno + atexit
 * table when none of these are set. */
/* #undef __THREAD_LOCAL_STORAGE */
/* #undef __THREAD_LOCAL_STORAGE_API */
/* #undef __THREAD_LOCAL_STORAGE_STACK_GUARD */

/* Plain `int errno;` (vs. `int* __errno()` indirection).
 *
 * LOAD-BEARING for the newlib ↔ picolibc errno bridge.  With this
 * set, picolibc's `libc/errno/errno.c` emits `int errno;` as a
 * strong global, and `runtime_stubs.c:__errno()` returns its
 * address.  Cross-toolchain archives built against newlib's
 * `#define errno (*__errno())` then resolve through the same
 * storage.  Unsetting this silently breaks the bridge — touch
 * picolibc.h and runtime_stubs.c together. */
#define __GLOBAL_ERRNO

/* Unused when __GLOBAL_ERRNO is set; leave undefined so accidental
 * references fail to compile. */
/* #undef __PICOLIBC_ERRNO_FUNCTION */

/* Single shared atexit table (single-threaded).  Without
 * `_ATEXIT_DYNAMIC_ALLOC`, the table is a fixed-size `.bss` array
 * — bare-metal sees at most a handful of C++ static-dtor entries. */
#define _GLOBAL_ATEXIT
/* #undef _ATEXIT_DYNAMIC_ALLOC */

/* ── Math ────────────────────────────────────────────────────────── */

/* `<math.h>` wrappers don't set `errno`. */
#define __IEEE_LIBM
/* #undef __MATH_ERRNO */

/* IEEEFP funcs (isnanl, finite, etc.) — off. */
/* #undef __IEEEFP_FUNCS */

/* Only consulted by picolibc's libm build, which we don't compile
 * (the cross toolchain's libm.a provides sqrt etc.).  Set to 0 so
 * the header parses cleanly. */
#define __OBSOLETE_MATH_FLOAT  0
#define __OBSOLETE_MATH_DOUBLE 0

/* ── Stdio ───────────────────────────────────────────────────────── */

/* Modern stdio: FILE*, fopen, vsnprintf, fprintf, … */
#define __TINY_STDIO

/* Format width: 'd' = double, 'f' = float, 'l' = long double,
 * 'i' = integer-only, 'm' = minimal.  'd' to keep accurate `%f` /
 * `%g` output. */
#define __IO_DEFAULT 'd'

/* C99 printf flags (`hh`, `ll`, `j`, `z`, `t`). */
#define __IO_C99_FORMATS

/* `%lld` etc. */
#define __IO_LONG_LONG

/* Full-precision Ryu path.  `__NANO_FORMATTED_IO` shaves a few KB
 * but produces visibly-wrong fractional output. */
/* #undef __IO_FLOAT_EXACT */
/* #undef __NANO_FORMATTED_IO */
/* #undef __IO_MINIMAL_LONG_LONG */

/* `%n` is the format-string attack vector; `%b` is a picolibc
 * extension we don't need. */
/* #undef __IO_PERCENT_N */
/* #undef __IO_PERCENT_B */

/* Other stdio knobs — all off for size. */
/* #undef __IO_POS_ARGS */
/* #undef __IO_WCHAR */
/* #undef __IO_LONG_DOUBLE */
/* #undef __IO_SMALL_ULTOA */
/* #undef __WIDE_ORIENT */

/* fseek-within-buffer optimisation.  Pure win. */
#define __FSEEK_OPTIMIZATION

/* Heavy multi-iovec write path — unused. */
/* #undef __FVWRITE_IN_STREAMIO */

/* Atomic ungetc — only matters across threads; covered by
 * `__SINGLE_THREAD`. */
/* #undef __ATOMIC_UNGETC */

/* Unbuffered stream optimisation — unused. */
/* #undef __UNBUF_STREAM_OPT */

/* ── Memory allocator ────────────────────────────────────────────── */

/* Consumer provides malloc/free/realloc/calloc as strong symbols.
 * Picolibc's nano-malloc / default malloc TUs stay off. */
/* #undef __NANO_MALLOC */
/* #undef __MALLOC_SMALL_BUCKET */

/* ── Locale / multibyte / wide char ──────────────────────────────── */

/* All off — text rendering is byte-oriented (UTF-8 input, SFNT
 * glyph indices); mbstowcs/wctomb are never called. */
/* #undef __MB_CAPABLE */
/* #undef __MB_EXTENDED_CHARSETS_ALL */
/* #undef __MB_EXTENDED_CHARSETS_UCS */
/* #undef __MB_EXTENDED_CHARSETS_ISO */
/* #undef __MB_EXTENDED_CHARSETS_WINDOWS */
/* #undef __MB_EXTENDED_CHARSETS_JIS */

/* ── Init / exit ─────────────────────────────────────────────────── */

/* GCC/clang emit `.init_array` / `.fini_array` for static
 * ctor/dtor registration on every supported target. */
#define __INIT_FINI_ARRAY
/* #undef __INIT_FINI_FUNCS */

/* Skip `__call_exitprocs` traversal — bare-metal `main()` doesn't
 * return.  `_exit()` is a stub in `runtime_stubs.c`. */
#define _LITE_EXIT

/* picocrt helper — we don't build picocrt. */
/* #undef _WANT_REGISTER_FINI */

/* ── OS surface ──────────────────────────────────────────────────── */

/* Consumer HALs own console / trap dispatch.  Nothing routed
 * through picolibc. */
/* #undef __HAVE_FCNTL */
/* #undef POSIX_CONSOLE */
/* #undef __SEMIHOST */
/* #undef __PICOCRT_ENABLE_MMU */
/* #undef __PICOCRT_RUNTIME_SIZE */

/* ── Compiler capabilities ───────────────────────────────────────── *
 *
 * Set statically rather than probed.  Meson would compile-test
 * these; the supported toolchain floor (GCC ≥ 4.8, clang ≥ 6)
 * makes the answers constant. */

/* GCC `__attribute__((alias))` — used for printf variant aliases. */
#define __strong_reference(sym, aliassym) \
    extern __typeof(sym) aliassym __attribute__((__alias__(#sym)))

/* `__attribute__((packed))` honours bitfield layout. */
#define __HAVE_BITFIELDS_IN_PACKED_STRUCTS

/* `_Complex` type — gates `<complex.h>`. */
#define __HAVE_COMPLEX

/* `-fno-builtin` / `-fno-tree-loop-distribute-patterns` prevent GCC
 * from rewriting `for(i;i<n;i++) dst[i]=src[i]` into a memcpy call
 * (which would recurse into the memcpy we're defining).  Picolibc's
 * call sites are hand-written asm or use inhibiting attributes
 * upstream, so we leave this off; turn on if a runaway memcpy
 * self-call ever surfaces. */
/* #undef __HAVE_CC_INHIBIT_LOOP_TO_LIBCALL */

/* ── Size vs. speed ──────────────────────────────────────────────── */

#define __PREFER_SIZE_OVER_SPEED
/* #undef __FAST_STRCMP */

/* ── Diagnostic verbosity ────────────────────────────────────────── */

/* assert() prints file:line:func.  Without this, it just calls
 * abort(). */
#define __ASSERT_VERBOSE

/* ── Newlib heritage version ─────────────────────────────────────── *
 *
 * Picolibc 1.8.x reports its newlib base as 4.3.0
 * (`meson.build:NEWLIB_VERSION`). */
#define _NEWLIB_VERSION       "4.3.0"
#define __NEWLIB__            4
#define __NEWLIB_MINOR__      3
#define __NEWLIB_PATCHLEVEL__ 0

/* ── Picolibc version ────────────────────────────────────────────── *
 *
 * Pinned to submodule tag 1.8.11.  Bump this block + the submodule
 * pointer together. */
#define _PICOLIBC_VERSION       "1.8.11"
#define _PICOLIBC__             1
#define _PICOLIBC_MINOR__       8
#define __PICOLIBC_VERSION__    "1.8.11"
#define __PICOLIBC__            1
#define __PICOLIBC_MINOR__      8
#define __PICOLIBC_PATCHLEVEL__ 11
