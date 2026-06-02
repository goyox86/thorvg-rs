# Bare-metal builds

`thorvg-sys` supports `target_os == "none"` targets out of the box.
This document explains how.

## TL;DR

On bare-metal targets, `thorvg-sys` vendors and compiles
[picolibc](https://github.com/picolibc/picolibc) as the libc, links
it as `libpicolibc.a`, and ships a small set of weak-symbol runtime
stubs for surface picolibc deliberately leaves to the consumer
(pthread no-ops, `getenv`, `_exit`, `__errno`, `_on_exit`, …).  The
cross toolchain's `libstdc++`, `libsupc++`, `libgcc`, and `libm`
are still used; only newlib is replaced.

Wired architectures: `riscv32`, `riscv64`, `aarch64`, `x86`,
`x86_64`, `powerpc[64]`, `mips[64]`, `sparc[64]`, `m68k`, `msp430`.
ARM (`arm`) is deliberately not wired yet — see
[Architecture support](#architecture-support).

## Why vendor picolibc

`thorvg` calls into the libc and the C++ standard library
extensively (`<string>`, `<algorithm>`, `<math.h>`, `<ctype.h>`,
`memcpy`, `bsearch`, `setjmp/longjmp`, `snprintf`, …).  On a hosted
target the system libc supplies all of that; on bare-metal nothing
does, and three options exist:

1. **Pull the cross toolchain's newlib `libc.a`.**  Newlib ships
   objects (`stack_protector.o`, …) that collide with HAL-defined
   symbols, and its per-toolchain quirks (errno macro shape,
   reentrancy globals, multilib variants) push the surface-area
   problem onto every consumer.
2. **Hand-write a weak-symbol shim.**  Works initially but grows
   monotonically: every new thorvg feature surfaces a new symbol,
   every new cross architecture needs hand-asm (`setjmp.S` per
   ISA), and the shim silently lies about correctness (UTF-8
   ctype, locale-aware sorting, accurate `printf("%f")`).
3. **Vendor a small, modern, BSD-licensed libc designed for
   embedded use.**  picolibc.  One source of truth, correct
   behaviour for everything thorvg actually needs, and arch
   portability is driven by picolibc's own `libc/machine/<arch>/`
   trees rather than this crate's hand-written asm.

We took option 3.

## Architecture support

`picolibc_machine_subdir` in `build.rs` is the single arch-policy
point.  It maps Rust's `CARGO_CFG_TARGET_ARCH` to picolibc's
`libc/machine/<dir>/`; any arch the helper maps to an existing
machine dir is built automatically.

| `target_arch` | picolibc dir | Status |
|---|---|---|
| `riscv32`, `riscv64` | `riscv` | wired |
| `aarch64` | `aarch64` | wired |
| `x86` | `i386` | wired |
| `x86_64` | `x86_64` | wired |
| `powerpc`, `powerpc64` | `powerpc` | wired |
| `mips`, `mips64` | `mips` | wired |
| `sparc`, `sparc64` | `sparc` | wired |
| `m68k` | `m68k` | wired |
| `msp430` | `msp430` | wired |
| `arm` | `arm` | **deferred** — see below |

Adding a new arch is a one-line edit to `picolibc_machine_subdir`,
provided picolibc's machine dir for that arch is a flat source set.

Only `riscv32imac-unknown-none-elf` has been exercised on real
hardware so far.  Other wired arches compile-link clean but are
not run-tested.

### The ARM caveat

Picolibc's `libc/machine/arm/` ships multiple ISA-variant `.S`
files (e.g. `setjmp.S` per armv4t / armv6m / armv7m / armv8m),
gated by meson on `-mcpu=`.  A flat directory walk picks up all
variants and link-errors on duplicate symbols.  Wiring ARM means
porting picolibc's meson selection rule into `build_picolibc`.
The current behaviour on `arm-*-none-*` targets is a build-time
panic with an actionable message.

## Build pipeline

`thorvg-sys/build.rs` is decomposed into named phases.  The
top-level orchestrator (`build_vendored_cc`) wires them together:

1. **Target classification** (`TargetInfo::from_env`) — three
   predicates (`is_bare_metal`, `is_hosted`, `is_msvc`) drive
   every downstream decision.
2. **Source / include enumeration** — feature-gated walks of the
   vendored thorvg tree (`collect_thorvg_sources`,
   `thorvg_include_dirs`).
3. **Compiler flag configuration** (`configure_thorvg_build`) —
   defines, meson-mirror flags (`-fno-exceptions`, `-fno-rtti`,
   …), bare-metal extras, multilib.
4. **Picolibc compile** (`build_picolibc`) — only on bare-metal.
   Walks `picolibc/libc/{ctype,string,stdlib,stdio,errno,search}/`
   plus `libc/machine/<arch>/` plus our `runtime_stubs/`, applies
   denylists, compiles into `libpicolibc.a`.
5. **Header isolation** (`apply_picolibc_header_isolation`) —
   only on bare-metal.  Strips system headers (`-nostdinc`),
   restores picolibc + compiler builtins + libstdc++ explicitly.
6. **thorvg compile + link directives** — final cc-rs invocation,
   `cargo:rustc-link-lib=` emission via
   `emit_runtime_link_directives`.

## Picolibc configuration

Upstream picolibc generates `picolibc.h` from `picolibc.h.in` via
meson at configure time.  We don't run meson; instead
`picolibc-config/picolibc.h` is a hand-authored static replacement
placed first on the include path, so every picolibc and thorvg TU
sees the same configuration.

Key choices (full list with rationale inline in `picolibc.h`):

| Knob | Setting | Why |
|---|---|---|
| `__SINGLE_THREAD` | set | No pthread on bare-metal; collapses every lock to a no-op |
| `__GLOBAL_ERRNO` | set | Plain `int errno;` global, required for the newlib bridge in `runtime_stubs/errno_bridge.c` |
| `__IEEE_LIBM` | set | `<math.h>` wrappers don't set `errno` |
| `__TINY_STDIO` | set | Modern stdio (FILE*, fopen, vsnprintf, fprintf) |
| `__IO_DEFAULT 'd'` | set | Double-precision `%f` / `%g` formatting |
| `__NANO_MALLOC` | unset | Consumer provides malloc/free as strong symbols |
| `__MB_CAPABLE` | unset | Text rendering is byte-oriented; no mbstowcs/wctomb |
| `_LITE_EXIT` | set | Skip heavy `__call_exitprocs` traversal |
| `__INIT_FINI_ARRAY` | set | Static ctor/dtor via `.init_array` / `.fini_array` |

Every `#cmakedefine` from the upstream template resolves to either
`#define` (set) or absence (unset).  Anything not covered fails
compilation loudly.

## Header isolation

Picolibc's headers must be the **only** libc headers any compiled
TU sees.  Mixing newlib headers with picolibc objects produces ABI
mismatches: `jmp_buf` size and layout differ, `FILE` / `errno`
positioning differs, function declarations clash.

Mechanism: `-nostdinc` strips ALL of GCC/Clang's default include
search paths (libc, compiler builtins, libstdc++).  We re-add
exactly what we want, in this order:

1. `picolibc-config/` — `<picolibc.h>` config, `<pthread.h>` stub
2. `libc/machine/<arch>/` — arch-specific machine overrides
3. `libc/{stdio,locale,stdlib}/` — internal cross-directory dirs
4. `libc/include/` — picolibc's public header tree
5. `-isystem <compiler-builtins>` — `<stdarg.h>`, `<stddef.h>`, …
6. `-isystem <libstdc++>` — `<string>`, `<algorithm>`, …

This applies to both picolibc's own TUs (in `build_picolibc`) and
thorvg's C++ TUs (in `apply_picolibc_header_isolation`).  The probe
helpers `cross_compiler_builtin_includes()` and
`cross_cxx_include_paths()` discover (5) and (6) from the cross
compiler via `-print-file-name=include` / `-E -x c++ -v`.

## Runtime stubs

Picolibc deliberately leaves a small surface to the OS / consumer:
pthread primitives, environment access, entropy, `_exit` / `raise`,
and a bridge symbol newlib-built archives expect.  We supply
defaults in `picolibc-config/runtime_stubs/` — one `.c` per
override unit, every symbol marked `__attribute__((weak))`.

| File | Symbols | Default behaviour |
|---|---|---|
| `pthread.c` | 10 pthread no-ops (`pthread_mutex_*`, `pthread_key_*`, `pthread_once`) | Return 0 / NULL; `pthread_once` invokes init the first time |
| `errno_bridge.c` | `__errno` | Returns address of picolibc's `errno` global so newlib-built archives resolve |
| `onexit.c` | `_on_exit` | No-op (atexit dispatch never fires — `main()` doesn't return) |
| `env.c` | `getenv` | Returns NULL (no environment) |
| `entropy.c` | `getentropy`, `arc4random` | `getentropy` returns -1 (no entropy source); `arc4random` returns 0 |
| `hal.c` | `_exit`, `raise` | Infinite halt loop / -1 |

## Overriding runtime stubs

Two override granularities work, and they compose.

### Per-symbol override (weak)

Define a strong replacement in your consumer crate:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn getentropy(buf: *mut u8, len: usize) -> i32 {
    if unsafe { my_trng_fill(buf, len) } { 0 } else { -1 }
}
```

The strong `getentropy` beats the weak one in
`libpicolibc.a(entropy.o)`.  The other symbol in `entropy.o`
(`arc4random`) still resolves to the weak stub.

### Per-TU override (linker-natural)

Provide strong replacements for **every** symbol in one of the
stub files.  The linker satisfies all references from your objects
and never pulls that `.o` from `libpicolibc.a` — the weak in-archive
stubs are silently absent from the final link.  No special
attribute needed; this is plain archive-linking semantics.

Example: full HAL replacement.

```rust
#[unsafe(no_mangle)]
pub extern "C" fn _exit(_status: core::ffi::c_int) -> ! {
    // HAL-specific halt / reset / breakpoint trap
    loop { core::hint::spin_loop() }
}

#[unsafe(no_mangle)]
pub extern "C" fn raise(_sig: core::ffi::c_int) -> core::ffi::c_int {
    -1
}
```

If you replace `_exit` but forget `raise`, the linker fails with
`undefined reference to raise` — a clear signal you missed one.

### Override-unit summary

| To override | Replace all of |
|---|---|
| HAL (halt / signal) | `_exit`, `raise` |
| Entropy | `getentropy`, `arc4random` |
| Environment | `getenv` |
| Threading | All 10 pthread functions (also flip `__SINGLE_THREAD` in `picolibc.h`) |
| `errno` bridge | `__errno` (must continue pointing at picolibc's `errno` global, or re-bridge to your own storage) |
| atexit dispatch | `_on_exit` (also un-denylist `exitprocs.c` in `build.rs` if you want picolibc's real implementation) |

## Cross-toolchain link surface

Picolibc replaces libc only.  The cross toolchain still supplies:

| Archive | Source | What it provides |
|---|---|---|
| `libstdc++.a` / `libc++.a` | cross toolchain | `std::string`, `<algorithm>`, … |
| `libsupc++.a` | cross toolchain | C++ ABI / EH glue (`operator new`, `__cxa_*`, vtables) |
| `libgcc.a` | cross toolchain | Compiler-rt equivalents (soft-float, divides, `_Unwind_*`) |
| `libm.a` | cross toolchain | sqrt, sin, cos, atan2 (used by thorvg's renderer) |
| `libc.a` | **NOT used** | picolibc replaces it; pulling newlib's `libc.a` drags colliding HAL symbols |

These are discovered by `cross_runtime_libs` via the cross
compiler's `-print-file-name=lib*.a` and emitted as
`cargo:rustc-link-search=` + `cargo:rustc-link-lib=static=`.
Multilib correctness is handled by `cross_toolchain_multilib_args`
(currently only required for RISC-V — cc-rs and GCC disagree on
`-march` naming).

## Adding a new architecture

For an arch whose picolibc machine dir is a flat source set:

1. Add an entry to `picolibc_machine_subdir` in `build.rs`:
   ```rust
   "your_arch" => Some("picolibc-dir-name"),
   ```
2. Build for `your_arch-unknown-none-elf` (or equivalent).
3. Validate at runtime if you have hardware.

If the arch needs per-ISA variant selection (ARM is the canonical
case), `build_picolibc`'s flat machine-dir walk will pick up all
variants and produce duplicate-symbol link errors.  In that case
the machine-source enumeration needs a per-arch hook in
`build_picolibc` mirroring picolibc's meson selection rule.

If the cross toolchain disagrees with cc-rs on multilib flag naming
(only RISC-V today), add a branch in `cross_toolchain_multilib_args`.
