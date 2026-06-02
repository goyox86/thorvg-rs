/*
 * runtime_stubs.c — strong-symbol stubs for libstdc++ / libsupc++ /
 * libc surface that picolibc deliberately doesn't provide and that
 * the thorvg-rs bare-metal contract treats as no-ops.
 *
 * Compiled into the picolibc static archive on bare-metal builds
 * when `picolibc_active` (see `thorvg-sys/build.rs`).  Replaces the
 * equivalent weak-symbol definitions formerly in
 * `thorvg/src/common/tvgLibcShim.cpp`.
 *
 * Why these stubs are STRONG (not weak):
 *
 *   The old `tvgLibcShim.cpp` used `__attribute__((weak))` to let
 *   consumer code override individual stubs without forking thorvg.
 *   Under picolibc we drop the weak attribute for two reasons:
 *
 *     1. No competing definitions exist on the link line.  We don't
 *        pull newlib's `libc.a`, picolibc itself doesn't define
 *        any of these (it deliberately leaves them as the OS
 *        contract for bare-metal targets), and the cross toolchain's
 *        `libsupc++.a` only *references* them.  So "weak" buys
 *        nothing.
 *     2. Strong symbols give clearer link-time diagnostics if a
 *        consumer accidentally duplicates a definition — the user
 *        gets a multiple-definition error instead of a silent
 *        substitution.
 *
 *   A downstream consumer that needs different behaviour (a real
 *   thread-safe `pthread_mutex_lock`, an entropy-providing
 *   `getentropy`, …) is expected to compile picolibc-config out of
 *   the link entirely or replace the relevant call sites at their
 *   layer.
 *
 * Coverage:
 *
 *   * pthread stubs — referenced by libsupc++'s `eh_alloc.o` and
 *     `eh_globals.o` (exception-handling glue compiled into every
 *     libstdc++-using binary).  Returning 0 / NULL is the
 *     well-defined behaviour for "no threading" mode; libsupc++'s
 *     own `__gthread_active_p()` short-circuits the locks anyway,
 *     but the symbols must still resolve.
 *   * `getenv` — bare-metal has no environment.  Returning NULL is
 *     the documented response for "variable unset" and disables
 *     every consumer's tunables path (libsupc++ probes
 *     `GLIBCXX_TUNABLES`).
 *   * `getentropy` — referenced by libstdc++'s `random.cc` (for
 *     `std::random_device`'s default ctor) and would be referenced
 *     by picolibc's `arc4random.c` if we compiled it (we don't —
 *     denylisted in build.rs).  Returning -1 with `errno = ENOSYS`
 *     is the documented "entropy source unavailable" signal;
 *     `std::random_device` falls through to a deterministic seed.
 *
 * Stack-protector and EH-globals symbols are NOT replicated here —
 * thorvg's build sets `-fno-stack-protector` and the toolchain's
 * libgcc / libsupc++ supply their own.
 */

#include <stddef.h>
#include <pthread.h>  /* our minimal stub header */

/* ── pthread no-ops ────────────────────────────────────────────── */

int pthread_mutex_init(pthread_mutex_t *m, const pthread_mutexattr_t *a)
{
    (void)m; (void)a;
    return 0;
}

int pthread_mutex_destroy(pthread_mutex_t *m)
{
    (void)m;
    return 0;
}

int pthread_mutex_lock(pthread_mutex_t *m)
{
    (void)m;
    return 0;
}

int pthread_mutex_unlock(pthread_mutex_t *m)
{
    (void)m;
    return 0;
}

int pthread_mutex_trylock(pthread_mutex_t *m)
{
    (void)m;
    return 0;
}

int pthread_key_create(pthread_key_t *k, void (*destructor)(void *))
{
    (void)k; (void)destructor;
    return 0;
}

int pthread_key_delete(pthread_key_t k)
{
    (void)k;
    return 0;
}

void *pthread_getspecific(pthread_key_t k)
{
    (void)k;
    return NULL;
}

int pthread_setspecific(pthread_key_t k, const void *v)
{
    (void)k; (void)v;
    return 0;
}

/* `pthread_once` MUST actually invoke `init_fn` on the first call.
 * Some consumers (rare, but possible — picolibc itself in modes we
 * don't enable, GLIBCXX paths under odd configurations) rely on the
 * first-call init dispatch.  Single-threaded semantics make `*o`
 * a plain "have we run" flag. */
int pthread_once(pthread_once_t *o, void (*init_fn)(void))
{
    if (*o == 0) {
        *o = 1;
        init_fn();
    }
    return 0;
}

/* ── Environment / entropy / misc ─────────────────────────────── */

char *getenv(const char *name)
{
    (void)name;
    return NULL;
}

/* `getentropy` is normally a syscall that fills `buf` with `len`
 * bytes of cryptographic-quality entropy.  We have none on bare
 * metal at this layer, so signal failure: callers (libstdc++'s
 * random.cc, …) treat -1 as "fall through to the next entropy
 * source" without trusting the buffer contents.  We zero the buffer
 * anyway to keep static analysers happy. */
int getentropy(void *buf, size_t len)
{
    unsigned char *p = (unsigned char *)buf;
    while (len--) *p++ = 0;
    return -1;
}

/* `arc4random` is BSD's keystream PRNG, used by libstdc++'s
 * `random.cc` as a fallback entropy source after `getentropy`
 * fails.  We blacklist picolibc's `arc4random.c` in build.rs (it
 * pulls a chacha-based re-seed loop that itself wants entropy),
 * so the symbol is still unresolved.  Return zero — it's a
 * documented legal output, and the caller treats it as a regular
 * sample.  Thorvg never instantiates `std::random_device`. */
unsigned int arc4random(void)
{
    return 0;
}

/* ── HAL surface picolibc deliberately leaves to the consumer ── *
 *
 * `_exit` and `raise` are the OS contract picolibc requires the
 * consumer to provide — they're the bottom of the abort / signal
 * / exit machinery.  Bare-metal `main()` never returns and we
 * have no signal handling, so the correct behaviour for both is
 * an infinite halt loop.  A real HAL build may override these
 * with breakpoint traps or HALT instructions; doing so requires
 * compiling picolibc-config out of the link, which is the
 * right escape hatch.
 *
 * Marked `__attribute__((noreturn))` to match their library
 * declarations and let the compiler dead-code subsequent code. */

__attribute__((noreturn)) void _exit(int status)
{
    (void)status;
    for (;;) {
        /* nothing */
    }
}

/* `raise` is non-noreturn in POSIX (it returns an int on success),
 * but on bare metal with no signal handling installed, a real
 * `raise(SIGABRT)` from `abort()` should halt.  Returning -1
 * (failure) lets callers fall through to `_exit`, which is the
 * actual halt loop. */
int raise(int sig)
{
    (void)sig;
    return -1;
}

/* ── newlib ↔ picolibc errno bridge ─────────────────────────── *
 *
 * The cross toolchain's pre-compiled archives — libm.a in
 * particular, plus assorted bits of libstdc++ — were built against
 * newlib's `<errno.h>`.  Newlib defines `errno` as a macro:
 *
 *     #define errno (*__errno())
 *
 * which means every `errno = E_FOO` and `if (errno == E_BAR)` site
 * in those archives emits a reference to the **function** `__errno`.
 *
 * Picolibc, by contrast, defines `errno` as a plain `int` global
 * (because we set `__GLOBAL_ERRNO` in `picolibc.h`).  So
 * picolibc's `libc/errno/errno.c` exports `int errno;` — not
 * `int *__errno(void)`.  The pre-compiled newlib-side TUs then
 * fail to link with an `undefined symbol: __errno`.
 *
 * Bridge: provide `__errno` here as a tiny accessor that returns
 * the address of picolibc's `errno` global.  Both worlds now see
 * the same underlying storage, errno values set from a math
 * routine in libm.a are readable by thorvg's C++ code through
 * picolibc's `<errno.h>`, and vice versa.
 *
 * Surfaced by the `gradient_linear` example bin: libm's float-to-
 * int conversions in gradient interpolation hit `__errno` on
 * domain errors. */
extern int errno;

int *__errno(void)
{
    return &errno;
}
