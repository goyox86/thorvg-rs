/*
 * runtime_stubs/entropy.c — `getentropy` and `arc4random` stubs.
 *
 * `getentropy` signals failure (-1) so callers (libstdc++ random.cc,
 * …) fall through to the next entropy source.  Buffer is zeroed
 * for static analyser cleanliness.
 *
 * `arc4random` is libstdc++'s fallback after `getentropy` fails;
 * returns 0 (a documented legal sample).  Picolibc's `arc4random.c`
 * is denylisted in build.rs (it pulls a chacha-based re-seed loop
 * that wants entropy itself).
 *
 * Consumers with TRNG hardware override one or both.  Return type
 * for `arc4random` matches picolibc's `<stdlib.h>` declaration
 * (`__uint32_t`) — GCC rejects signature mismatches.
 */

#include <stddef.h>
#include <stdlib.h>

__attribute__((weak))
int getentropy(void *buf, size_t len)
{
    unsigned char *p = (unsigned char *)buf;
    while (len--) *p++ = 0;
    return -1;
}

__attribute__((weak))
__uint32_t arc4random(void)
{
    return 0;
}
