/*
 * pthread.h — minimal stub for libstdc++'s `<bits/gthr-default.h>`
 * when picolibc replaces newlib on bare-metal builds.
 *
 * Why this file exists:
 *
 *   picolibc, unlike newlib, does *not* ship a `<pthread.h>`.  But
 *   the cross toolchain's libstdc++ headers (`<thread>`, `<mutex>`,
 *   `<chrono>`) transitively include `<pthread.h>` via
 *   `<bits/gthr.h>` → `<bits/gthr-default.h>`, even on builds where
 *   thorvg never instantiates `std::thread`.  Without a header on
 *   the include path the C++ compile fails before it even reaches
 *   the active code paths.
 *
 *   We're already in the world of `-fno-threadsafe-statics` (set by
 *   `build.rs` on bare metal) and `__SINGLE_THREAD` (set in
 *   `picolibc.h`) — every actual locking call is dead-code-eliminated.
 *   So the only job of this header is to make `<bits/gthr-default.h>`
 *   *compile*, with libstdc++'s weak-symbol-based
 *   `__gthread_active_p()` returning `false` at run time.
 *
 * How libstdc++ decides "are we threaded":
 *
 *   GCC's gthr-default.h checks the link-time address of
 *   `pthread_cancel`:
 *
 *       static void *const __gthread_active_ptr
 *           = __extension__ (void *) &__gthrw_(pthread_cancel);
 *       return __gthread_active_ptr != 0;
 *
 *   `__gthrw_(pthread_cancel)` resolves to a weak external reference.
 *   When the symbol stays unresolved at link time (our case),
 *   `&__gthrw_pthread_cancel` is 0, and `__gthread_active_p()` is
 *   false → every locking call is bypassed.
 *
 *   So `pthread_cancel` (and friends) must be DECLARED in this
 *   header (so the libstdc++ headers parse), but NEVER DEFINED in
 *   any object file we link (so the weak reference stays NULL).
 *
 * What does get inline-stubbed:
 *
 *   A handful of pthread functions are called by libstdc++ *even
 *   when `__gthread_active_p()` is false* — typically as part of
 *   one-time initialisation of function-local statics, or via
 *   `__cxa_guard_*` paths that `-fno-threadsafe-statics` doesn't
 *   fully suppress.  The pre-picolibc `tvgLibcShim.cpp` stubbed
 *   these with `__attribute__((weak))` no-op definitions; we move
 *   the same stubs here as `static inline` so any TU that pulls
 *   in `<pthread.h>` automatically gets them, dead-code-eliminated
 *   when unused.
 *
 *   The functions in this category:
 *     - `pthread_mutex_init` / `_destroy` / `_lock` / `_unlock` /
 *       `_trylock`
 *     - `pthread_key_create` / `_delete` / `getspecific` /
 *       `setspecific`
 *     - `pthread_once`  (must actually call `init_fn` the first
 *                       time so static-init dispatch works)
 *
 * Type layouts:
 *
 *   All pthread types here are `int`-sized.  libstdc++ only ever
 *   passes them by pointer to (a) the inline stubs above (which
 *   ignore the contents), or (b) the unresolved weak externs
 *   (which never get called).  Zero-init via the global static
 *   initialiser pattern (`= PTHREAD_MUTEX_INITIALIZER`) is the only
 *   write that ever lands at run time, and `int = 0` covers that.
 */

#ifndef _PTHREAD_H_
#define _PTHREAD_H_

#include <sys/cdefs.h>
#include <sys/types.h>  /* time_t for pthread_*_timed* signatures */
#include <time.h>       /* struct timespec */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Types ──────────────────────────────────────────────────────── */

typedef int pthread_t;
typedef int pthread_key_t;
typedef int pthread_once_t;
typedef int pthread_mutex_t;
typedef int pthread_cond_t;
typedef int pthread_rwlock_t;
typedef int pthread_attr_t;
typedef int pthread_mutexattr_t;
typedef int pthread_condattr_t;
typedef int pthread_rwlockattr_t;

/* ── Static-initialiser macros ─────────────────────────────────── *
 *
 * All zero — picolibc's `__SINGLE_THREAD` mode means these objects
 * are never inspected.  The macros exist for source compatibility
 * with C++ code that does `pthread_mutex_t m = PTHREAD_MUTEX_INITIALIZER;` */

#define PTHREAD_MUTEX_INITIALIZER   0
#define PTHREAD_ONCE_INIT           0
#define PTHREAD_COND_INITIALIZER    0
#define PTHREAD_RWLOCK_INITIALIZER  0

/* Mutex kind constants — values from POSIX.  libstdc++'s `<mutex>`
 * passes these to `pthread_mutexattr_settype` (a weak-undefined
 * function below), so they need defined values but the actual
 * integers don't matter. */
#define PTHREAD_MUTEX_NORMAL        0
#define PTHREAD_MUTEX_RECURSIVE     1
#define PTHREAD_MUTEX_ERRORCHECK    2
#define PTHREAD_MUTEX_DEFAULT       PTHREAD_MUTEX_NORMAL

/* Detach states for `pthread_attr_setdetachstate`. */
#define PTHREAD_CREATE_JOINABLE     0
#define PTHREAD_CREATE_DETACHED     1

/* Cancel state for `pthread_setcancelstate`. */
#define PTHREAD_CANCEL_ENABLE       0
#define PTHREAD_CANCEL_DISABLE      1

/* ── Weak-undefined externs ────────────────────────────────────── *
 *
 * Declared so libstdc++ headers compile; deliberately NEVER defined
 * in any object file we link.  `__gthread_active_p()`'s linker-
 * address probe of `pthread_cancel` then resolves to NULL, putting
 * libstdc++ into its single-thread fallback code path.
 *
 * If a downstream consumer one day actually wants pthreads, they
 * can define any subset of these as strong symbols in their crate
 * and `__gthread_active_p()` will flip back to true — but at that
 * point picolibc's `__SINGLE_THREAD` is also wrong, and the right
 * fix is a different libc config, not poking holes here. */

extern int pthread_create(pthread_t *, const pthread_attr_t *,
                          void *(*)(void *), void *);
extern int pthread_cancel(pthread_t);
extern int pthread_join(pthread_t, void **);
extern int pthread_detach(pthread_t);
extern void pthread_exit(void *) __attribute__((noreturn));
extern pthread_t pthread_self(void);
extern int pthread_equal(pthread_t, pthread_t);

extern int pthread_attr_init(pthread_attr_t *);
extern int pthread_attr_destroy(pthread_attr_t *);
extern int pthread_attr_setdetachstate(pthread_attr_t *, int);

extern int pthread_mutexattr_init(pthread_mutexattr_t *);
extern int pthread_mutexattr_destroy(pthread_mutexattr_t *);
extern int pthread_mutexattr_settype(pthread_mutexattr_t *, int);

extern int pthread_mutex_timedlock(pthread_mutex_t *,
                                   const struct timespec *);

extern int pthread_cond_init(pthread_cond_t *, const pthread_condattr_t *);
extern int pthread_cond_destroy(pthread_cond_t *);
extern int pthread_cond_signal(pthread_cond_t *);
extern int pthread_cond_broadcast(pthread_cond_t *);
extern int pthread_cond_wait(pthread_cond_t *, pthread_mutex_t *);
extern int pthread_cond_timedwait(pthread_cond_t *, pthread_mutex_t *,
                                  const struct timespec *);

extern int pthread_rwlock_rdlock(pthread_rwlock_t *);
extern int pthread_rwlock_tryrdlock(pthread_rwlock_t *);
extern int pthread_rwlock_wrlock(pthread_rwlock_t *);
extern int pthread_rwlock_trywrlock(pthread_rwlock_t *);
extern int pthread_rwlock_unlock(pthread_rwlock_t *);

extern int pthread_setcancelstate(int, int *);
extern int pthread_getschedparam(pthread_t, int *, void *);
extern int pthread_setschedparam(pthread_t, int, const void *);
extern int pthread_default_stacksize_np(size_t, size_t *);

/* ── sched.h surface libstdc++ also takes weak references to ─── *
 *
 * Picolibc ships `<sched.h>`, but the declarations for these
 * symbols are gated on `_POSIX_THREADS` / `_POSIX_PRIORITY_SCHEDULING`
 * — macros we deliberately do not define.  libstdc++'s
 * `<bits/gthr-default.h>` references them anyway through the
 * weak `__gthrw_(name)` mechanism (see the `__gthrw_(sched_yield)`
 * etc. usage), so they need to be DECLARED on the include path for
 * the C++ compile to succeed; their absence at link time keeps
 * `__gthread_active_p()` returning 0.
 *
 * Declared here rather than in `<sched.h>` because the latter is
 * picolibc-shipped and we don't want to monkey-patch the submodule.
 * pthread.h is our authored stub, so additional weak externs go
 * here. */
extern int sched_yield(void);
extern int sched_get_priority_min(int);
extern int sched_get_priority_max(int);

/* ── Stubbed-strong externs (real defs in runtime_stubs.c) ───── *
 *
 * These are the pthread functions that libsupc++.a's pre-compiled
 * `eh_alloc.o` / `eh_globals.o` (exception-handling glue baked into
 * every libstdc++-using binary) call as undefined externals at
 * link time.  Strong definitions live in
 * `picolibc-config/runtime_stubs.c`, which `build.rs` compiles into
 * the picolibc static archive.
 *
 * An earlier draft had them as `static inline` no-ops here, which
 * worked for thorvg's own C++ TUs but cannot satisfy already-
 * compiled archive objects — the linker doesn't see header inlines. */

extern int pthread_mutex_init(pthread_mutex_t *,
                              const pthread_mutexattr_t *);
extern int pthread_mutex_destroy(pthread_mutex_t *);
extern int pthread_mutex_lock(pthread_mutex_t *);
extern int pthread_mutex_unlock(pthread_mutex_t *);
extern int pthread_mutex_trylock(pthread_mutex_t *);
extern int pthread_key_create(pthread_key_t *, void (*)(void *));
extern int pthread_key_delete(pthread_key_t);
extern void *pthread_getspecific(pthread_key_t);
extern int pthread_setspecific(pthread_key_t, const void *);
extern int pthread_once(pthread_once_t *, void (*)(void));

#ifdef __cplusplus
}
#endif

#endif /* _PTHREAD_H_ */
