# picolibc-config

Hand-authored configuration headers for the vendored
[picolibc](../picolibc/) submodule.

Upstream picolibc generates these from `.in` templates via meson at
configure time.  We don't run meson; the headers here are checked in
and placed *first* on the compile-time include path by
`thorvg-sys/build.rs`, so every picolibc and thorvg translation unit
sees the same static configuration.

## Files

| File | Upstream counterpart | Purpose |
|---|---|---|
| `picolibc.h` | `picolibc.h.in` (resolved by meson) | All ~50 picolibc compile-time knobs: errno shape, threading mode, stdio family, math errno policy, locale support, version macros |
| `pthread.h` | none (picolibc doesn't ship one) | Minimal stub so libstdc++ headers parse under `__SINGLE_THREAD` |
| `runtime_stubs/*.c` | none | Weak-symbol stubs for surface picolibc doesn't ship (pthread, getenv, getentropy, `_exit`, `__errno`, `_on_exit`) |

## runtime_stubs/

One `.c` per override unit so a consumer's strong replacement causes
the linker to skip exactly that one TU from `libpicolibc.a`:

| File | Symbols | When to override |
|---|---|---|
| `pthread.c` | `pthread_{mutex,key,…}_*`, `pthread_once` | Real threading (also flip `__SINGLE_THREAD` in picolibc.h) |
| `errno_bridge.c` | `__errno` | Custom errno storage (must re-bridge to picolibc's `errno`) |
| `onexit.c` | `_on_exit` | Want real atexit dispatch (also un-denylist `exitprocs.c`) |
| `env.c` | `getenv` | Real environment surface |
| `entropy.c` | `getentropy`, `arc4random` | TRNG hardware |
| `hal.c` | `_exit`, `raise` | Breakpoint trap, watchdog reset, power-off |

All stubs use `__attribute__((weak))`.  Consumer crates supply
strong replacements via the usual mechanism:

```rust
// somewhere in the consumer crate
#[unsafe(no_mangle)]
pub extern "C" fn _exit(_status: core::ffi::c_int) -> ! {
    // HAL-specific halt / reset / breakpoint trap
    loop { core::hint::spin_loop() }
}
```

## Editing

Knobs are documented inline in `picolibc.h` next to each `#define` /
`#undef` site.  When upgrading the picolibc submodule, re-read
`picolibc/picolibc.h.in` for any new `#cmakedefine` lines and add
them here with an explicit choice (set or unset — do not leave them
implicitly absent without a comment, the build relies on every knob
being covered intentionally).

## Why hand-authored vs. generated

* `build.rs` already probes the cross toolchain (sysroot, multilib,
  runtime libs); also spawning meson or replicating its knob-resolution
  logic (cross-compile test programs, target introspection, option-file
  parsing) would more than double its complexity.
* The knob set is small (~30 set, ~20 deliberately off).  Hand-
  authoring is faster to review and yields a single auditable artifact.
* Submodule bumps stay clean: the picolibc tree is untouched on disk,
  no generated files leak into the working copy.
