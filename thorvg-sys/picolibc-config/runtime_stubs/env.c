/*
 * runtime_stubs/env.c — `getenv` stub.
 *
 * No environment on bare-metal.  NULL is the documented response
 * for "variable unset" and disables consumer tunables paths
 * (libsupc++ probes `GLIBCXX_TUNABLES`).
 */

#include <stddef.h>

__attribute__((weak))
char *getenv(const char *name)
{
    (void)name;
    return NULL;
}
