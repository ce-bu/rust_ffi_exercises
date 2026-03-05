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
extern "C"
{
#endif

    /* ── Ex01: C math functions ─────────────────────────────────── */

    int c_add(int a, int b);
    int c_multiply(int a, int b);
    double c_distance(double x1, double y1, double x2, double y2);
    int c_abs(int x);

    /* ── Ex07: C callback helpers ───────────────────────────────── */

    void c_for_each(const int32_t *array, size_t len,
                    void (*callback)(int32_t, void *),
                    void *user_data);
    int32_t c_transform(int32_t value,
                        int32_t (*transform)(int32_t));

    /* ── Ex11: C++ vtable dispatch ──────────────────────────────── */

    typedef struct ShapeVTable
    {
        double (*area)(const void *data);
        double (*perimeter)(const void *data);
        void (*destroy)(void *data);
    } ShapeVTable;

    typedef struct CShape
    {
        void *data;
        const ShapeVTable *vtable;
    } CShape;

    double cpp_total_area(const CShape *shapes, size_t count);
    void cpp_destroy_shapes(CShape *shapes, size_t count);

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
#define CDB_OK 0
#define CDB_ERR_INVALID -1
#define CDB_ERR_NOT_FOUND -2
#define CDB_ERR_OVERFLOW -3

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

#define SESSION_OK 0
#define SESSION_ERR_NULL -1
#define SESSION_ERR_STATE -2
#define SESSION_ERR_OVERFLOW -3

    int session_create(Session **out);
    int session_set_option(Session *s, const char *key, const char *value);
    int session_get_option(Session *s, const char *key,
                           char *out_value, size_t out_len);
    int session_connect(Session *s, const char *host);
    int session_is_connected(Session *s);
    int session_send(Session *s, const unsigned char *data, size_t len);
    int session_recv(Session *s, unsigned char *buf,
                     size_t buf_len, size_t *out_len);
    int session_disconnect(Session *s);
    void session_destroy(Session *s);

    /* ── Ex25: calling C++ virtual functions (direct vtable) ───── */

    /**
     * Dual-interface handle for polymorphic C++ objects with MI.
     * Returned by the factory functions below.
     *
     * `shape`   = IShape*   (primary base, also allocation address)
     * `labeled` = ILabeled* (secondary base, pointer-adjusted)
     */
    typedef struct CppDualHandle
    {
        void *shape;
        void *labeled;
    } CppDualHandle;

    CppDualHandle cpp_vt_create_rect(const char *label,
                                     double x, double y,
                                     double w, double h);
    CppDualHandle cpp_vt_create_circle(const char *label,
                                       double cx, double cy,
                                       double r);

    /* Introspection helpers (for Rust test verification) */
    void *cpp_vt_read_slot(const void *interface_ptr, int32_t slot);
    ptrdiff_t cpp_vt_offset_to_top(const void *interface_ptr);
    size_t cpp_vt_sizeof_rect(void);
    size_t cpp_vt_sizeof_circle(void);
    size_t cpp_vt_ilabeled_offset_in_rect(void);
    size_t cpp_vt_ilabeled_offset_in_circle(void);

    /* ── Ex26: C++ exceptions across FFI boundaries ──────────── */

    int32_t cpp_ex_get_error(char *out_msg, size_t msg_len,
                             char *out_type, size_t type_len,
                             int32_t *out_code);
    void    cpp_ex_clear_error(void);

    int32_t cpp_ex_divide(double a, double b, double *out);
    int32_t cpp_ex_parse_int(const char *s, size_t len, int64_t *out);
    int32_t cpp_ex_sqrt(double x, double *out);
    int32_t cpp_ex_process_data(const uint8_t *data, size_t len,
                                int32_t *out_checksum);
    int32_t cpp_ex_trigger_unknown(void);

    typedef int32_t (*CppExMapFn)(double input, double *output, void *ctx);
    int32_t cpp_ex_map_array(const double *input, double *output,
                             size_t len, CppExMapFn map_fn, void *ctx);

    /* ── Ex27: wrapping C++ classes & STL types ─────────────── */

    typedef struct CppStringStack CppStringStack;

    CppStringStack *cpp_stk_new(void);
    void            cpp_stk_destroy(CppStringStack *s);
    CppStringStack *cpp_stk_clone(const CppStringStack *s);

    int32_t cpp_stk_push(CppStringStack *s, const char *str, size_t len);
    int32_t cpp_stk_pop(CppStringStack *s, char *out_buf, size_t buf_len,
                         size_t *out_len);
    int32_t cpp_stk_peek(const CppStringStack *s, const char **out_ptr,
                          size_t *out_len);
    size_t  cpp_stk_size(const CppStringStack *s);

    int32_t cpp_stk_join(const CppStringStack *s,
                          const char *sep, size_t sep_len,
                          char *out_buf, size_t buf_len, size_t *out_len);
    int32_t cpp_stk_push_many(CppStringStack *s,
                               const char *const *strings,
                               const size_t *lengths, size_t count);

    typedef void (*CppStkIterFn)(const char *str, size_t len, void *ctx);
    void cpp_stk_for_each(const CppStringStack *s,
                           CppStkIterFn callback, void *ctx);

    CppStringStack *cpp_stk_from_csv(const char *csv, size_t len);

#ifdef __cplusplus
}
#endif

#endif /* EXERCISES_H */
