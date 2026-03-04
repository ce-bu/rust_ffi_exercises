/*
 * csrc/ex12_blocking.c
 *
 * Pre-provided blocking C function for Exercise 12 (Async interop).
 * Simulates a slow I/O or compute operation.
 */

#ifdef _WIN32
#include <windows.h>
#else
#include <unistd.h>
#endif

#include <stddef.h>

/* Simulates a blocking computation that takes ~10 ms. */
int c_slow_compute(int input) {
#ifdef _WIN32
    Sleep(10);
#else
    usleep(10000);
#endif
    return input * input + 1;
}
