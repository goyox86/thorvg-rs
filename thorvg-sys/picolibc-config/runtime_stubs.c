/*
 * runtime_stubs.c — strong-symbol stubs for libstdc++ / libsupc++ /
 * libc surface that picolibc deliberately doesn't provide and that
 * the bare-metal contract treats as no-ops.
 *
 * Compiled into the picolibc static archive (see build.rs) so it
 * shares picolibc's include path and multilib configuration.
 *
 * Strong (not weak) symbols: nothing else on the link line defines
 * these (we don't pull newlib's libc.a, picolibc leaves them as the
 * OS contract, and libsupc++ only references them), so weak buys
 * nothing.  Strong gives clearer link-time diagnostics on
 * accidental duplicates.  A consumer that needs real implementations
 * compiles picolibc-config out of the link and supplies its own.
 *
 * Coverage:
 *   * pthread stubs — referenced unconditionally by libsupc++'s
 *     `eh_alloc.o` / `eh_globals.o` even though
 *     `__gthread_active_p()` short-circuits the locks.
 *   * `getenv` — no environment on bare-metal; returns NULL.
 *   * `getentropy` — no entropy source; returns -1.
 *   * `arc4random` — fallback after `getentropy` fails; returns 0.
 *   * `_exit` / `raise` — picolibc's OS contract; halt loop.
 *   * `__errno` — bridge for newlib-built libm/libstdc++ archives.
 *   * `_on_exit` — picolibc's internal exit-registration primitive.
 *
 * Stack-protector and EH-globals symbols are NOT here — thorvg
 * builds with `-fno-stack-protector` and the toolchain's
 * libgcc / libsupc++ supply EH globals.
 */

#include <stddef.h>
#include <pthread.h>      /* our minimal stub header */
#include "local-onexit.h" /* picolibc:libc/stdlib/local-onexit.h —
                           * `enum pico_onexit_kind` and
                           * `union on_exit_func` for `_on_exit`.
                           * Resolved by build.rs adding
                           * libc/stdlib/ to the include path. */

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

/* `pthread_once` MUST actually invoke `init_fn` on the first call
 * — static-init dispatch paths reach it.  Single-threaded
 * semantics make `*o` a plain "have we run" flag. */
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

/* Signal "entropy source unavailable" with -1.  Callers
 * (libstdc++'s random.cc, …) treat -1 as "fall through to the next
 * source" without trusting buffer contents.  Zero the buffer to
 * keep static analysers happy. */
int getentropy(void *buf, size_t len)
{
    unsigned char *p = (unsigned char *)buf;
    while (len--) *p++ = 0;
    return -1;
}

/* `arc4random` is libstdc++'s fallback after `getentropy` fails.
 * Picolibc's `arc4random.c` pulls a chacha-based re-seed loop we
 * don't want; we provide a stub so the symbol resolves.  Zero is
 * a documented legal output; consumers treat it as a regular sample.
 *
 * Return type matches picolibc's `<stdlib.h>` declaration (pulled
 * transitively via `local-onexit.h`) — `__uint32_t` is `unsigned
 * long int` on rv32 vs. `unsigned int` (different typedef, same
 * size), which GCC rejects on signature mismatch. */
__uint32_t arc4random(void)
{
    return 0;
}

/* ── HAL surface picolibc leaves to the consumer ─────────────── *
 *
 * `_exit` and `raise` are picolibc's required OS contract — bottom
 * of the abort / signal / exit machinery.  Bare-metal `main()`
 * doesn't return and we have no signal handling, so both halt.
 * A real HAL build overrides them by compiling picolibc-config
 * out of the link. */

__attribute__((noreturn)) void _exit(int status)
{
    (void)status;
    for (;;) {
        /* nothing */
    }
}

/* Non-noreturn per POSIX.  Returning -1 lets callers fall through
 * to `_exit` (the actual halt loop). */
int raise(int sig)
{
    (void)sig;
    return -1;
}

/* ── newlib ↔ picolibc errno bridge ─────────────────────────── *
 *
 * The cross toolchain's pre-compiled archives (libm.a, parts of
 * libstdc++) were built against newlib's `<errno.h>`, which defines
 *
 *     #define errno (*__errno())
 *
 * so every `errno = E_FOO` in those archives emits a reference to
 * the **function** `__errno`.  Picolibc's `__GLOBAL_ERRNO` mode
 * exports `int errno;` as a plain global — no `__errno` function,
 * so newlib-side TUs fail to link.
 *
 * Bridge: provide `__errno` as a tiny accessor returning the
 * address of picolibc's `errno` global.  Both worlds then see the
 * same underlying storage. */

/* DO NOT `#include <errno.h>` here.  Newlib's `<errno.h>` would
 * `#define errno (*__errno())` and textually rewrite the line
 * below into `extern int (*__errno());`, breaking the bridge.
 * The manual declaration resolves to picolibc's strong `int errno;`
 * symbol from `libc/errno/errno.c`. */
extern int errno;

int *__errno(void)
{
    return &errno;
}

/* ── picolibc's internal atexit primitive ───────────────────── *
 *
 * Picolibc routes every exit-registration API (`atexit()`,
 * `on_exit()`, `__cxa_atexit()`) through `_on_exit`, whose real
 * implementation lives in `libc/stdlib/exitprocs.c`.  We denylist
 * that file (~1 KB BSS for dynamic atexit storage we don't need
 * — bare-metal `main()` doesn't return) and stub `_on_exit` here.
 * `__INIT_FINI_ARRAY` in picolibc.h routes C++ static destructors
 * through `.fini_array` rather than `_on_exit`, so the bypass is
 * total.  Picolibc's `atexit.c` is still compiled and links
 * against this stub.
 *
 * Signature follows picolibc's `local-onexit.h` (included at the
 * top); a submodule bump that changes the types surfaces as a
 * compile error rather than silent ABI drift. */

int _on_exit(enum pico_onexit_kind kind, union on_exit_func func, void *arg)
{
    (void)kind;
    (void)func;
    (void)arg;
    return 0;
}
