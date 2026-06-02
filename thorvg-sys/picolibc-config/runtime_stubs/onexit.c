/*
 * runtime_stubs/onexit.c — stub for picolibc's internal exit-
 * registration primitive.
 *
 * Picolibc routes every exit-registration API (atexit(), on_exit(),
 * __cxa_atexit()) through `_on_exit`.  The real implementation
 * (`libc/stdlib/exitprocs.c`) is denylisted in build.rs — ~1 KB of
 * BSS for dynamic atexit storage we don't need on bare-metal
 * (`main()` doesn't return; `__INIT_FINI_ARRAY` in picolibc.h
 * routes C++ static destructors through `.fini_array` instead).
 *
 * Signature follows picolibc's `local-onexit.h` so a submodule bump
 * that changes the types surfaces as a compile error.
 */

#include "local-onexit.h"

__attribute__((weak))
int _on_exit(enum pico_onexit_kind kind, union on_exit_func func, void *arg)
{
    (void)kind; (void)func; (void)arg;
    return 0;
}
