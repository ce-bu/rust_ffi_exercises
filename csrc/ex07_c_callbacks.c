/*
 * csrc/ex07_c_callbacks.c
 *
 * Pre-provided C functions for Exercise 07 (Callbacks).
 * Demonstrates C code that *accepts* function-pointer callbacks.
 */

#include <stddef.h>
#include <stdint.h>

/* Calls `callback(element, user_data)` for every element in `array`. */
void c_for_each(const int32_t *array,
                size_t len,
                void (*callback)(int32_t, void *),
                void *user_data)
{
    for (size_t i = 0; i < len; ++i) {
        callback(array[i], user_data);
    }
}

/* Applies a transformation function to `value` and returns the result. */
int32_t c_transform(int32_t value,
                    int32_t (*transform)(int32_t))
{
    return transform(value);
}
