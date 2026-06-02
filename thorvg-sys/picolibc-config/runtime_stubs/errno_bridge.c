/*
 * runtime_stubs/errno_bridge.c — `__errno()` accessor.
 *
 * The cross toolchain's pre-compiled archives (libm.a, parts of
 * libstdc++) were built against newlib's `<errno.h>`, which defines
 *
 *     #define errno (*__errno())
 *
 * Picolibc's `__GLOBAL_ERRNO` mode exports `int errno;` as a plain
 * global — no `__errno` function — so newlib-side TUs fail to link.
 * This accessor returns the address of picolibc's `errno` global so
 * both worlds resolve through the same storage.
 *
 * Weak, but overriding without re-bridging picolibc's `errno` will
 * break the round-trip.  Touch with care.
 */

/* DO NOT `#include <errno.h>` here — newlib's would `#define errno
 * (*__errno())` and textually rewrite the line below into
 * `extern int (*__errno());`, breaking the bridge. */
extern int errno;

__attribute__((weak))
int *__errno(void)
{
    return &errno;
}
