/*
 * runtime_stubs/pthread.c — no-op pthread stubs for libsupc++ EH glue.
 *
 * Referenced unconditionally by libsupc++'s `eh_alloc.o` /
 * `eh_globals.o`, even though `__gthread_active_p()` short-circuits
 * the locks at run time (see pthread.h for the weak-extern probe).
 *
 * All symbols weak — a consumer with real threading supplies strong
 * replacements.  `__SINGLE_THREAD` in picolibc.h is the matching
 * config knob; flip both together for actual threading.
 */

#include <stddef.h>
#include <pthread.h>

__attribute__((weak))
int pthread_mutex_init(pthread_mutex_t *m, const pthread_mutexattr_t *a)
{
    (void)m; (void)a;
    return 0;
}

__attribute__((weak))
int pthread_mutex_destroy(pthread_mutex_t *m)
{
    (void)m;
    return 0;
}

__attribute__((weak))
int pthread_mutex_lock(pthread_mutex_t *m)
{
    (void)m;
    return 0;
}

__attribute__((weak))
int pthread_mutex_unlock(pthread_mutex_t *m)
{
    (void)m;
    return 0;
}

__attribute__((weak))
int pthread_mutex_trylock(pthread_mutex_t *m)
{
    (void)m;
    return 0;
}

__attribute__((weak))
int pthread_key_create(pthread_key_t *k, void (*destructor)(void *))
{
    (void)k; (void)destructor;
    return 0;
}

__attribute__((weak))
int pthread_key_delete(pthread_key_t k)
{
    (void)k;
    return 0;
}

__attribute__((weak))
void *pthread_getspecific(pthread_key_t k)
{
    (void)k;
    return NULL;
}

__attribute__((weak))
int pthread_setspecific(pthread_key_t k, const void *v)
{
    (void)k; (void)v;
    return 0;
}

/* `pthread_once` MUST actually invoke `init_fn` on the first call —
 * static-init dispatch paths reach it.  Single-threaded semantics
 * make `*o` a plain "have we run" flag. */
__attribute__((weak))
int pthread_once(pthread_once_t *o, void (*init_fn)(void))
{
    if (*o == 0) {
        *o = 1;
        init_fn();
    }
    return 0;
}
