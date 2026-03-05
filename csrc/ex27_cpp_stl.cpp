/*
 * csrc/ex27_cpp_stl.cpp
 *
 * A non-virtual C++ class (`StringStack`) that uses STL
 * containers (std::vector<std::string>) internally.  Wrapped
 * with extern "C" functions to demonstrate the common patterns
 * for exposing C++ classes to Rust through a C ABI:
 *
 * - Opaque pointer (create/destroy)
 * - std::string ↔ const char* + size_t
 * - Caller-provided buffer for string output
 * - Borrowed pointer return (peek — pointer into C++ memory!)
 * - Copy constructor → clone
 * - Factory function  → from_csv
 * - Callback-based iteration
 * - Batch insertion (push_many)
 *
 * Error handling uses simple return codes; no thread-local
 * storage (contrast with ex26's pattern).
 */

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <cstdlib>
#include <string>
#include <vector>
#include <sstream>
#include <new>
#include <stdexcept>

/* ══════════════════════════════════════════════════════════════
 * C++ class — not exported directly
 * ══════════════════════════════════════════════════════════════ */

class CppStringStack
{
    std::vector<std::string> items_;

public:
    CppStringStack() = default;
    CppStringStack(const CppStringStack &) = default;
    CppStringStack &operator=(const CppStringStack &) = default;

    void push(const char *s, size_t len)
    {
        items_.emplace_back(s, len);
    }

    /* throws std::out_of_range if empty */
    std::string pop()
    {
        if (items_.empty())
            throw std::out_of_range("pop from empty stack");
        std::string top = std::move(items_.back());
        items_.pop_back();
        return top;
    }

    /* Returns a BORROWED reference – valid until next mutation */
    const std::string &peek() const
    {
        if (items_.empty())
            throw std::out_of_range("peek at empty stack");
        return items_.back();
    }

    size_t size() const { return items_.size(); }
    bool empty() const { return items_.empty(); }

    std::string join(const char *sep, size_t sep_len) const
    {
        std::string result;
        const std::string separator(sep, sep_len);
        for (size_t i = 0; i < items_.size(); ++i)
        {
            if (i > 0)
                result += separator;
            result += items_[i];
        }
        return result;
    }

    /* Factory: split CSV into items */
    static CppStringStack from_csv(const char *csv, size_t len)
    {
        CppStringStack s;
        std::string input(csv, len);
        std::istringstream iss(input);
        std::string token;
        while (std::getline(iss, token, ','))
        {
            /* trim leading/trailing spaces */
            size_t start = token.find_first_not_of(' ');
            size_t end = token.find_last_not_of(' ');
            if (start != std::string::npos)
                s.items_.push_back(token.substr(start, end - start + 1));
        }
        return s;
    }

    const std::vector<std::string> &items() const { return items_; }
};

/* ══════════════════════════════════════════════════════════════
 * extern "C" wrappers
 * ══════════════════════════════════════════════════════════════ */

extern "C"
{

/* Error codes */
#define CPP_STK_OK 0
#define CPP_STK_ERR_EMPTY -1
#define CPP_STK_ERR_BUF -2 /* output buffer too small */
#define CPP_STK_ERR_NULL -3
#define CPP_STK_ERR_OOM -4 /* allocation failure */
#define CPP_STK_ERR_OTHER -99

    /* ── Lifecycle ──────────────────────────────────────────────── */

    CppStringStack *cpp_stk_new(void)
    {
        try
        {
            return new CppStringStack();
        }
        catch (...)
        {
            return nullptr;
        }
    }

    void cpp_stk_destroy(CppStringStack *s)
    {
        delete s; /* delete nullptr is safe */
    }

    CppStringStack *cpp_stk_clone(const CppStringStack *s)
    {
        if (!s)
            return nullptr;
        try
        {
            return new CppStringStack(*s); /* C++ copy constructor */
        }
        catch (...)
        {
            return nullptr;
        }
    }

    /* ── Push ───────────────────────────────────────────────────── */

    int32_t cpp_stk_push(CppStringStack *s, const char *str, size_t len)
    {
        if (!s)
            return CPP_STK_ERR_NULL;
        try
        {
            s->push(str, len);
            return CPP_STK_OK;
        }
        catch (const std::bad_alloc &)
        {
            return CPP_STK_ERR_OOM;
        }
        catch (...)
        {
            return CPP_STK_ERR_OTHER;
        }
    }

    /* ── Pop — copies string into caller's buffer ───────────────── */

    /**
     * @param out_buf   Caller-provided buffer (may be NULL to query length).
     * @param buf_len   Size of out_buf.
     * @param out_len   Receives the actual string length (excl. NUL).
     *                  Always written (even on ERR_BUF) so the caller can
     *                  retry with a larger buffer.
     */
    int32_t cpp_stk_pop(CppStringStack *s,
                        char *out_buf, size_t buf_len, size_t *out_len)
    {
        if (!s || !out_len)
            return CPP_STK_ERR_NULL;
        try
        {
            /* Peek first — don't remove until we know the buffer fits. */
            const std::string &top = s->peek();
            *out_len = top.size();
            if (!out_buf || buf_len < top.size() + 1)
                return CPP_STK_ERR_BUF;
            std::memcpy(out_buf, top.c_str(), top.size() + 1);
            s->pop(); /* now safe to remove */
            return CPP_STK_OK;
        }
        catch (const std::out_of_range &)
        {
            *out_len = 0;
            return CPP_STK_ERR_EMPTY;
        }
        catch (...)
        {
            *out_len = 0;
            return CPP_STK_ERR_OTHER;
        }
    }

    /* ── Peek — returns a BORROWED pointer ──────────────────────── */

    /**
     * Sets *out_ptr to point DIRECTLY into the C++ std::string's
     * internal buffer.  The pointer is valid ONLY until the next
     * push, pop, or destroy call on this stack.
     *
     * This is the cheapest return mechanism (zero-copy) but places
     * a lifetime burden on the caller.
     */
    int32_t cpp_stk_peek(const CppStringStack *s,
                         const char **out_ptr, size_t *out_len)
    {
        if (!s || !out_ptr || !out_len)
            return CPP_STK_ERR_NULL;
        try
        {
            const std::string &ref = s->peek();
            *out_ptr = ref.c_str();
            *out_len = ref.size();
            return CPP_STK_OK;
        }
        catch (const std::out_of_range &)
        {
            *out_ptr = nullptr;
            *out_len = 0;
            return CPP_STK_ERR_EMPTY;
        }
        catch (...)
        {
            *out_ptr = nullptr;
            *out_len = 0;
            return CPP_STK_ERR_OTHER;
        }
    }

    /* ── Size ───────────────────────────────────────────────────── */

    size_t cpp_stk_size(const CppStringStack *s)
    {
        return s ? s->size() : 0;
    }

    /* ── Join — copies joined string to caller buffer ───────────── */

    int32_t cpp_stk_join(const CppStringStack *s,
                         const char *sep, size_t sep_len,
                         char *out_buf, size_t buf_len, size_t *out_len)
    {
        if (!s || !out_len)
            return CPP_STK_ERR_NULL;
        try
        {
            std::string result = s->join(sep, sep_len);
            *out_len = result.size();
            if (!out_buf || buf_len < result.size() + 1)
                return CPP_STK_ERR_BUF;
            std::memcpy(out_buf, result.c_str(), result.size() + 1);
            return CPP_STK_OK;
        }
        catch (...)
        {
            *out_len = 0;
            return CPP_STK_ERR_OTHER;
        }
    }

    /* ── Push many (batch insertion) ────────────────────────────── */

    int32_t cpp_stk_push_many(CppStringStack *s,
                              const char *const *strings,
                              const size_t *lengths,
                              size_t count)
    {
        if (!s || (!strings && count > 0))
            return CPP_STK_ERR_NULL;
        try
        {
            for (size_t i = 0; i < count; ++i)
            {
                s->push(strings[i], lengths[i]);
            }
            return CPP_STK_OK;
        }
        catch (const std::bad_alloc &)
        {
            return CPP_STK_ERR_OOM;
        }
        catch (...)
        {
            return CPP_STK_ERR_OTHER;
        }
    }

    /* ── Iteration via callback ─────────────────────────────────── */

    typedef void (*CppStkIterFn)(const char *str, size_t len, void *ctx);

    void cpp_stk_for_each(const CppStringStack *s,
                          CppStkIterFn callback, void *ctx)
    {
        if (!s || !callback)
            return;
        for (const auto &item : s->items())
        {
            callback(item.c_str(), item.size(), ctx);
        }
    }

    /* ── Factory — parse CSV ────────────────────────────────────── */

    CppStringStack *cpp_stk_from_csv(const char *csv, size_t len)
    {
        if (!csv)
            return nullptr;
        try
        {
            auto *p = new CppStringStack(CppStringStack::from_csv(csv, len));
            return p;
        }
        catch (...)
        {
            return nullptr;
        }
    }

} /* extern "C" */
