/*
 * csrc/ex26_cpp_exceptions.cpp
 *
 * C++ functions that throw various exceptions, wrapped in
 * extern "C" functions with try/catch.  Demonstrates the
 * canonical pattern for safe C++ exception → Rust Result
 * conversion.
 *
 * KEY RULE: An exception that propagates through an `extern "C"`
 * boundary is **undefined behavior**.  Every extern "C" function
 * MUST catch all exceptions.
 *
 * Error info is stored in a thread-local struct so Rust can
 * retrieve the message after checking the return code (similar
 * to errno / GetLastError).
 */

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <cmath>
#include <cstdlib>
#include <cstdio>
#include <stdexcept>
#include <string>
#include <typeinfo>

/* ══════════════════════════════════════════════════════════════
 * Thread-local exception storage
 * ══════════════════════════════════════════════════════════════ */

struct ExceptionInfo {
    char    message[512];
    char    type_name[128];
    int32_t code;
    bool    active;
};

static thread_local ExceptionInfo tl_exc = {};

static void store_exc(int32_t code, const char *msg, const char *type) {
    tl_exc.code = code;
    std::strncpy(tl_exc.message, msg, sizeof(tl_exc.message) - 1);
    tl_exc.message[sizeof(tl_exc.message) - 1] = '\0';
    std::strncpy(tl_exc.type_name, type, sizeof(tl_exc.type_name) - 1);
    tl_exc.type_name[sizeof(tl_exc.type_name) - 1] = '\0';
    tl_exc.active = true;
}

static void clear_exc() { tl_exc = {}; }

/* ══════════════════════════════════════════════════════════════
 * Custom C++ exception class
 * ══════════════════════════════════════════════════════════════ */

class ProcessingError : public std::runtime_error {
    int detail_code_;
public:
    ProcessingError(int code, const std::string &msg)
        : std::runtime_error(msg), detail_code_(code) {}
    int detail_code() const { return detail_code_; }
};

/* ══════════════════════════════════════════════════════════════
 * Internal C++ functions (may throw)
 * ══════════════════════════════════════════════════════════════ */

namespace internal {

double divide(double a, double b) {
    if (b == 0.0)
        throw std::domain_error("division by zero");
    return a / b;
}

int64_t parse_int(const char *s, size_t len) {
    if (!s || len == 0)
        throw std::invalid_argument("empty string");
    std::string str(s, len);
    char *end = nullptr;
    long long val = std::strtoll(str.c_str(), &end, 10);
    if (*end != '\0')
        throw std::invalid_argument(
            std::string("invalid integer: '") + str + "'");
    return static_cast<int64_t>(val);
}

double sqrt_checked(double x) {
    if (x < 0.0)
        throw std::domain_error("sqrt of negative number");
    return std::sqrt(x);
}

int32_t process_data(const uint8_t *data, size_t len) {
    if (!data)
        throw std::invalid_argument("null data pointer");
    if (len == 0)
        throw ProcessingError(1001, "empty data");
    if (len > 4096)
        throw ProcessingError(1002, "data too large (max 4096)");
    int32_t checksum = 0;
    for (size_t i = 0; i < len; ++i) {
        if (data[i] == 0xFF)
            throw ProcessingError(1003,
                "invalid byte 0xFF at offset " + std::to_string(i));
        checksum += static_cast<int32_t>(data[i]);
    }
    return checksum;
}

void throw_integer() {
    throw 42; // non-std::exception — caught only by catch(...)
}

} // namespace internal

/* ══════════════════════════════════════════════════════════════
 * extern "C" API
 * ══════════════════════════════════════════════════════════════ */

extern "C" {

/* Error codes */
#define CPP_EX_OK            0
#define CPP_EX_ERR_DOMAIN   -1
#define CPP_EX_ERR_INVALID  -2
#define CPP_EX_ERR_CUSTOM   -3
#define CPP_EX_ERR_UNKNOWN  -99

/* ── Error info retrieval ───────────────────────────────────── */

/**
 * Retrieve the last exception's details.
 *
 * @return 1 if an error was present, 0 if no error.
 */
int32_t cpp_ex_get_error(char *out_msg, size_t msg_len,
                         char *out_type, size_t type_len,
                         int32_t *out_code)
{
    if (!tl_exc.active) return 0;
    if (out_msg && msg_len > 0) {
        std::strncpy(out_msg, tl_exc.message, msg_len - 1);
        out_msg[msg_len - 1] = '\0';
    }
    if (out_type && type_len > 0) {
        std::strncpy(out_type, tl_exc.type_name, type_len - 1);
        out_type[type_len - 1] = '\0';
    }
    if (out_code) *out_code = tl_exc.code;
    return 1;
}

void cpp_ex_clear_error(void) { clear_exc(); }

/* ── Wrapped operations ─────────────────────────────────────── */

/*
 * PATTERN: Every extern "C" function follows this template:
 *
 *   clear_exc();
 *   try {
 *       ... call internal C++ code ...
 *       return CPP_EX_OK;
 *   } catch (const SpecificException& e) {
 *       store_exc(SPECIFIC_CODE, e.what(), "SpecificException");
 *       return SPECIFIC_CODE;
 *   } catch (const std::exception& e) {
 *       store_exc(CPP_EX_ERR_UNKNOWN, e.what(), typeid(e).name());
 *       return CPP_EX_ERR_UNKNOWN;
 *   } catch (...) {
 *       store_exc(CPP_EX_ERR_UNKNOWN, "unknown exception", "...");
 *       return CPP_EX_ERR_UNKNOWN;
 *   }
 */

int32_t cpp_ex_divide(double a, double b, double *out) {
    clear_exc();
    try {
        *out = internal::divide(a, b);
        return CPP_EX_OK;
    } catch (const std::domain_error &e) {
        store_exc(CPP_EX_ERR_DOMAIN, e.what(), "std::domain_error");
        return CPP_EX_ERR_DOMAIN;
    } catch (const std::exception &e) {
        store_exc(CPP_EX_ERR_UNKNOWN, e.what(), typeid(e).name());
        return CPP_EX_ERR_UNKNOWN;
    } catch (...) {
        store_exc(CPP_EX_ERR_UNKNOWN, "unknown exception", "...");
        return CPP_EX_ERR_UNKNOWN;
    }
}

int32_t cpp_ex_parse_int(const char *s, size_t len, int64_t *out) {
    clear_exc();
    try {
        *out = internal::parse_int(s, len);
        return CPP_EX_OK;
    } catch (const std::invalid_argument &e) {
        store_exc(CPP_EX_ERR_INVALID, e.what(), "std::invalid_argument");
        return CPP_EX_ERR_INVALID;
    } catch (const std::exception &e) {
        store_exc(CPP_EX_ERR_UNKNOWN, e.what(), typeid(e).name());
        return CPP_EX_ERR_UNKNOWN;
    } catch (...) {
        store_exc(CPP_EX_ERR_UNKNOWN, "unknown exception", "...");
        return CPP_EX_ERR_UNKNOWN;
    }
}

int32_t cpp_ex_sqrt(double x, double *out) {
    clear_exc();
    try {
        *out = internal::sqrt_checked(x);
        return CPP_EX_OK;
    } catch (const std::domain_error &e) {
        store_exc(CPP_EX_ERR_DOMAIN, e.what(), "std::domain_error");
        return CPP_EX_ERR_DOMAIN;
    } catch (const std::exception &e) {
        store_exc(CPP_EX_ERR_UNKNOWN, e.what(), typeid(e).name());
        return CPP_EX_ERR_UNKNOWN;
    } catch (...) {
        store_exc(CPP_EX_ERR_UNKNOWN, "unknown exception", "...");
        return CPP_EX_ERR_UNKNOWN;
    }
}

int32_t cpp_ex_process_data(const uint8_t *data, size_t len,
                            int32_t *out_checksum)
{
    clear_exc();
    try {
        *out_checksum = internal::process_data(data, len);
        return CPP_EX_OK;
    } catch (const ProcessingError &e) {
        store_exc(CPP_EX_ERR_CUSTOM, e.what(), "ProcessingError");
        return CPP_EX_ERR_CUSTOM;
    } catch (const std::invalid_argument &e) {
        store_exc(CPP_EX_ERR_INVALID, e.what(), "std::invalid_argument");
        return CPP_EX_ERR_INVALID;
    } catch (const std::exception &e) {
        store_exc(CPP_EX_ERR_UNKNOWN, e.what(), typeid(e).name());
        return CPP_EX_ERR_UNKNOWN;
    } catch (...) {
        store_exc(CPP_EX_ERR_UNKNOWN, "unknown exception", "...");
        return CPP_EX_ERR_UNKNOWN;
    }
}

/* Throws a non-std::exception (plain int). */
int32_t cpp_ex_trigger_unknown(void) {
    clear_exc();
    try {
        internal::throw_integer();
        return CPP_EX_OK;
    } catch (const std::exception &e) {
        store_exc(CPP_EX_ERR_UNKNOWN, e.what(), typeid(e).name());
        return CPP_EX_ERR_UNKNOWN;
    } catch (int code) {
        char msg[64];
        std::snprintf(msg, sizeof(msg), "caught int: %d", code);
        store_exc(CPP_EX_ERR_UNKNOWN, msg, "int");
        return CPP_EX_ERR_UNKNOWN;
    } catch (...) {
        store_exc(CPP_EX_ERR_UNKNOWN, "unknown non-std exception", "...");
        return CPP_EX_ERR_UNKNOWN;
    }
}

/* ── Callback integration ───────────────────────────────────── */

/**
 * Apply `map_fn(input[i]) → output[i]` for each element.
 *
 * If the callback returns non-zero, processing stops and
 * that error code is returned.  The entire loop is also
 * wrapped in try/catch in case C++ code throws.
 */
typedef int32_t (*CppExMapFn)(double input, double *output, void *ctx);

int32_t cpp_ex_map_array(const double *input, double *output,
                         size_t len,
                         CppExMapFn map_fn, void *ctx)
{
    clear_exc();
    try {
        for (size_t i = 0; i < len; ++i) {
            int32_t rc = map_fn(input[i], &output[i], ctx);
            if (rc != 0) {
                char msg[128];
                std::snprintf(msg, sizeof(msg),
                    "callback error %d at index %zu", rc, i);
                store_exc(rc, msg, "callback_error");
                return rc;
            }
        }
        return CPP_EX_OK;
    } catch (const std::exception &e) {
        store_exc(CPP_EX_ERR_UNKNOWN, e.what(), typeid(e).name());
        return CPP_EX_ERR_UNKNOWN;
    } catch (...) {
        store_exc(CPP_EX_ERR_UNKNOWN, "unknown exception in map", "...");
        return CPP_EX_ERR_UNKNOWN;
    }
}

} /* extern "C" */
