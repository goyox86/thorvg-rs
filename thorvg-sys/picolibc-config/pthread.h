/*
 * pthread.h — minimal stub for libstdc++'s `<bits/gthr-default.h>`.
 *
 * Picolibc does not ship `<pthread.h>`, but the cross toolchain's
 * libstdc++ headers (`<thread>`, `<mutex>`, `<chrono>`) transitively
 * include it via `<bits/gthr.h>` → `<bits/gthr-default.h>`, even on
 * builds that never instantiate `std::thread`.  Without a header on
 * the include path the C++ compile fails before reaching any active
 * code path.
 *
 * `__SINGLE_THREAD` (set in picolibc.h) and `-fno-threadsafe-statics`
 * (set in build.rs) make every locking call dead code, so this
 * header just needs to *parse*.
 *
 * libstdc++'s "are we threaded" probe takes the link-time address of
 * `pthread_cancel` as a weak external:
 *
 *   static void *const __gthread_active_ptr
 *       = __extension__ (void *) &__gthrw_(pthread_cancel);
 *   return __gthread_active_ptr != 0;
 *
 * If `pthread_cancel` resolves to NULL at link time,
 * `__gthread_active_p()` returns false and every locking call is
 * bypassed.  So `pthread_cancel` (and friends below) must be
 * DECLARED here but NEVER DEFINED anywhere we link.
 *
 * A handful of pthread functions get called even when
 * `__gthread_active_p()` is false (one-time static-init dispatch
 * paths that `-fno-threadsafe-statics` doesn't fully suppress, plus
 * libsupc++'s `eh_alloc.o` / `eh_globals.o` references).  Strong
 * implementations live in `runtime_stubs.c`; they're declared at
 * the bottom of this header.
 *
 * Type layouts: all pthread types here are `int`-sized.  libstdc++
 * only ever passes them by pointer to inline stubs (which ignore
 * the contents) or to unresolved weak externs (never called).
 * Zero-init via `PTHREAD_*_INITIALIZER` is the only write that
 * lands at run time, and `int = 0` covers that.
 */

#ifndef _PTHREAD_H_
#define _PTHREAD_H_

#include <sys/cdefs.h>
#include <sys/types.h>  /* time_t for pthread_*_timed* */
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
 * Values are never inspected under `__SINGLE_THREAD`.  Macros exist
 * for source compatibility with C++ code that does
 * `pthread_mutex_t m = PTHREAD_MUTEX_INITIALIZER;` */

#define PTHREAD_MUTEX_INITIALIZER   0
#define PTHREAD_ONCE_INIT           0
#define PTHREAD_COND_INITIALIZER    0
#define PTHREAD_RWLOCK_INITIALIZER  0

/* Mutex kind constants — values from POSIX.  libstdc++'s `<mutex>`
 * passes these to `pthread_mutexattr_settype` (a weak-undefined
 * function below); integers must be defined but the actual values
 * don't matter. */
#define PTHREAD_MUTEX_NORMAL        0
#define PTHREAD_MUTEX_RECURSIVE     1
#define PTHREAD_MUTEX_ERRORCHECK    2
#define PTHREAD_MUTEX_DEFAULT       PTHREAD_MUTEX_NORMAL

#define PTHREAD_CREATE_JOINABLE     0
#define PTHREAD_CREATE_DETACHED     1

#define PTHREAD_CANCEL_ENABLE       0
#define PTHREAD_CANCEL_DISABLE      1

/* ── Weak-undefined externs ────────────────────────────────────── *
 *
 * Declared so libstdc++ headers compile; deliberately NEVER defined
 * in any object we link.  Keeps `__gthread_active_p()` returning 0. */

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

/* ── sched.h surface ─────────────────────────────────────────── *
 *
 * Picolibc ships `<sched.h>`, but the declarations for these are
 * gated on `_POSIX_THREADS` / `_POSIX_PRIORITY_SCHEDULING` macros
 * we don't define.  libstdc++ references them via `__gthrw_(name)`
 * weak externs anyway; declared here rather than in `<sched.h>`
 * to avoid monkey-patching the submodule. */
extern int sched_yield(void);
extern int sched_get_priority_min(int);
extern int sched_get_priority_max(int);

/* ── Strong externs (defined in runtime_stubs.c) ─────────────── *
 *
 * Called from pre-compiled libsupc++.a objects (`eh_alloc.o`,
 * `eh_globals.o`) as undefined externals at link time.  Cannot be
 * `static inline` here — the linker doesn't see header inlines. */

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
