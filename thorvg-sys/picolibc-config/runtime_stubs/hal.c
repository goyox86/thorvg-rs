/*
 * runtime_stubs/hal.c — `_exit` and `raise`.
 *
 * Picolibc's required OS contract — bottom of the abort / signal /
 * exit machinery.  Default is an infinite halt loop: bare-metal
 * `main()` doesn't return, no signal handling is installed.
 *
 * Consumers with a real HAL (breakpoint trap, watchdog reset,
 * power-off) supply strong replacements at the consumer crate
 * level.
 */

__attribute__((noreturn, weak))
void _exit(int status)
{
    (void)status;
    for (;;) {
        /* nothing */
    }
}

/* Non-noreturn per POSIX.  Returning -1 lets callers fall through
 * to `_exit` (the actual halt loop). */
__attribute__((weak))
int raise(int sig)
{
    (void)sig;
    return -1;
}
