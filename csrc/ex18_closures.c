/*
 * csrc/ex18_closures.c
 *
 * C helper functions for Exercise 18 (Rust Closures from C).
 *
 * Each function receives a function-pointer + opaque context pointer
 * and invokes the callback.  The Rust side packages closures behind
 * this (fn-ptr, void*) pair.
 */

#include <stddef.h>
#include <stdint.h>

/* ── Fn-style: called multiple times, context never consumed ── */

int32_t c_apply_twice(int32_t a, int32_t b,
                      int32_t (*f)(int32_t, void *),
                      void *ctx)
{
    /* Call the closure-trampoline twice and sum the results. */
    return f(a, ctx) + f(b, ctx);
}

/* ── FnMut-style: called multiple times, context may mutate ─── */

void c_generate(int32_t *out, size_t len,
                int32_t (*next)(void *),
                void *ctx)
{
    for (size_t i = 0; i < len; ++i) {
        out[i] = next(ctx);
    }
}

/* ── FnOnce-style: called exactly once, context is consumed ─── */

int32_t c_call_once(int32_t (*f)(void *), void *ctx)
{
    return f(ctx);
}

/* ── Bonus: higher-order – apply a unary fn to each element ─── */

void c_map_array(const int32_t *input, int32_t *output, size_t len,
                 int32_t (*f)(int32_t, void *),
                 void *ctx)
{
    for (size_t i = 0; i < len; ++i) {
        output[i] = f(input[i], ctx);
    }
}
