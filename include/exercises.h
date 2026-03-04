/**
 * @file exercises.h
 * @brief Shared header for FFI exercise C/C++ helpers.
 */

#ifndef EXERCISES_H
#define EXERCISES_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Ex01: C math functions ─────────────────────────────────── */

int    c_add(int a, int b);
int    c_multiply(int a, int b);
double c_distance(double x1, double y1, double x2, double y2);
int    c_abs(int x);

/* ── Ex07: C callback helpers ───────────────────────────────── */

void    c_for_each(const int32_t *array, size_t len,
                   void (*callback)(int32_t, void *),
                   void *user_data);
int32_t c_transform(int32_t value,
                    int32_t (*transform)(int32_t));

/* ── Ex11: C++ vtable dispatch ──────────────────────────────── */

typedef struct ShapeVTable {
    double (*area)(const void *data);
    double (*perimeter)(const void *data);
    void   (*destroy)(void *data);
} ShapeVTable;

typedef struct CShape {
    void             *data;
    const ShapeVTable *vtable;
} CShape;

double cpp_total_area(const CShape *shapes, size_t count);
void   cpp_destroy_shapes(CShape *shapes, size_t count);

/* ── Ex12: blocking C function ──────────────────────────────── */

int c_slow_compute(int input);

/* ── Ex18: invoking Rust closures from C ────────────────────── */

/**
 * Call `f(a, ctx) + f(b, ctx)`.  The callback is invoked twice
 * (Fn-style: shared, non-mutating context).
 */
int32_t c_apply_twice(int32_t a, int32_t b,
                      int32_t (*f)(int32_t, void *),
                      void *ctx);

/**
 * Fill `out[0..len]` by calling `next(ctx)` for each element.
 * The callback is invoked multiple times and may mutate its
 * context (FnMut-style).
 */
void c_generate(int32_t *out, size_t len,
                int32_t (*next)(void *),
                void *ctx);

/**
 * Call `f(ctx)` exactly once and return its result.
 * The callback may consume its context (FnOnce-style).
 */
int32_t c_call_once(int32_t (*f)(void *), void *ctx);

/**
 * Apply `f(input[i], ctx)` to produce `output[i]` for each element.
 * The callback is invoked `len` times (Fn-style).
 */
void c_map_array(const int32_t *input, int32_t *output, size_t len,
                 int32_t (*f)(int32_t, void *),
                 void *ctx);

/* ── Ex17: opaque-handle C "database" library ───────────────── */

/**
 * Opaque handle — callers only see `CdbHandle *`, never the
 * internal layout.
 */
typedef struct CdbHandle CdbHandle;

/** Status codes returned by cdb_* functions. */
#define CDB_OK            0
#define CDB_ERR_INVALID  -1
#define CDB_ERR_NOT_FOUND -2
#define CDB_ERR_OVERFLOW  -3

/**
 * Open an in-memory key-value store.
 * @param path  Ignored in this toy implementation, but must be
 *              non-NULL and non-empty (simulates a file path).
 * @return Handle on success, NULL on failure.
 */
CdbHandle *cdb_open(const char *path);

/**
 * Insert or update a key-value pair.
 * @return CDB_OK or an error code.
 */
int cdb_put(CdbHandle *db, const char *key, const char *value);

/**
 * Retrieve the value for @p key into a caller-provided buffer.
 * @param out_value  Destination buffer.
 * @param out_len    Size of the destination buffer (including NUL).
 * @return CDB_OK, CDB_ERR_NOT_FOUND, or CDB_ERR_OVERFLOW.
 */
int cdb_get(CdbHandle *db, const char *key,
            char *out_value, size_t out_len);

/**
 * Delete a key.  Returns CDB_ERR_NOT_FOUND if the key is absent.
 */
int cdb_delete(CdbHandle *db, const char *key);

/**
 * Return the number of stored key-value pairs.
 */
size_t cdb_count(CdbHandle *db);

/**
 * Close the handle and release all resources.
 * The handle must not be used after this call.
 */
int cdb_close(CdbHandle *db);

/* ── Ex24: C-owned opaque session handle ─────────────────────── */

/**
 * Opaque session handle — allocated by C, returned via
 * out-parameter.  Every function takes `Session *` as the
 * first argument (like C++ `this`).
 */
typedef struct Session Session;

#define SESSION_OK            0
#define SESSION_ERR_NULL     -1
#define SESSION_ERR_STATE    -2
#define SESSION_ERR_OVERFLOW -3

int  session_create(Session **out);
int  session_set_option(Session *s, const char *key, const char *value);
int  session_get_option(Session *s, const char *key,
                        char *out_value, size_t out_len);
int  session_connect(Session *s, const char *host);
int  session_is_connected(Session *s);
int  session_send(Session *s, const unsigned char *data, size_t len);
int  session_recv(Session *s, unsigned char *buf,
                  size_t buf_len, size_t *out_len);
int  session_disconnect(Session *s);
void session_destroy(Session *s);

#ifdef __cplusplus
}
#endif

#endif /* EXERCISES_H */
