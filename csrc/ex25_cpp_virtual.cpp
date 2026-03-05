/*
 * csrc/ex25_cpp_virtual.cpp
 *
 * C++ polymorphic objects for Exercise 25.
 *
 * KEY DESIGN: There are **no trampoline functions** for virtual
 * methods.  The Rust side reads the vtable pointer (vptr) directly
 * from the object and invokes function pointers by slot index —
 * exploiting the Itanium C++ ABI vtable layout.
 *
 * The only `extern "C"` functions are:
 *   • Factory functions (create objects)
 *   • Introspection helpers (verify layout assumptions in tests)
 *
 * All virtual method calls (area, perimeter, scale, bounding_box,
 * get_label, set_label) AND destruction go through direct vtable
 * reads from Rust.
 *
 * ┌──────────────────────────────────────────────────────────┐
 * │ WARNING: This relies on the Itanium C++ ABI.            │
 * │ It works with GCC and Clang on Linux, macOS, BSD, etc.  │
 * │ It does NOT work with MSVC (different vtable layout).   │
 * └──────────────────────────────────────────────────────────┘
 */

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <cmath>
#include <cstdlib>

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

/* ══════════════════════════════════════════════════════════════
 * Abstract C++ interfaces
 *
 * IShape vtable slots (after 2 destructor entries):
 *   slot 0: complete destructor  (D1)
 *   slot 1: deleting destructor  (D0)
 *   slot 2: area()
 *   slot 3: perimeter()
 *   slot 4: bounding_box()
 *   slot 5: scale()
 *
 * ILabeled vtable slots:
 *   slot 0: complete destructor  (D1)
 *   slot 1: deleting destructor  (D0)
 *   slot 2: get_label()
 *   slot 3: set_label()
 * ══════════════════════════════════════════════════════════════ */

class IShape
{
public:
    virtual ~IShape() = default;

    virtual double area() const = 0;
    virtual double perimeter() const = 0;

    /* Out-parameters: writes bounding box into the four pointers. */
    virtual void bounding_box(double *out_x, double *out_y,
                              double *out_w, double *out_h) const = 0;

    /* In/out parameter: scales geometry by `factor`.
     * On entry  *inout_area holds the caller's current area.
     * On exit   *inout_area holds the new post-scale area.        */
    virtual void scale(double factor, double *inout_area) = 0;
};

class ILabeled
{
public:
    virtual ~ILabeled() = default;

    /* Out-parameters: copies label into out_buf (max buf_len bytes
     * incl. NUL).  Sets *out_len to the string length (w/o NUL).
     * Returns 0 on success, -1 if buffer too small.               */
    virtual int get_label(char *out_buf, size_t buf_len,
                          size_t *out_len) const = 0;

    /* In-parameter: replaces the current label. */
    virtual void set_label(const char *new_label) = 0;
};

/* ══════════════════════════════════════════════════════════════
 * Concrete: LabeledRect   (IShape + ILabeled — multiple inheritance)
 *
 * Object layout (Itanium ABI, 64-bit):
 *
 *   offset  0: vptr → IShape vtable   (primary base)
 *   offset  8: vptr → ILabeled vtable (secondary base)
 *   offset 16: double x_
 *   offset 24: double y_
 *   offset 32: double w_
 *   offset 40: double h_
 *   offset 48: char   label_[64]
 *   ───────── total: 112 bytes ─────────
 * ══════════════════════════════════════════════════════════════ */

class LabeledRect : public IShape, public ILabeled
{
    double x_, y_, w_, h_;
    char label_[64];

public:
    LabeledRect(const char *lbl, double x, double y, double w, double h)
        : x_(x), y_(y), w_(w), h_(h)
    {
        std::strncpy(label_, lbl, sizeof(label_) - 1);
        label_[sizeof(label_) - 1] = '\0';
    }

    double area() const override { return w_ * h_; }
    double perimeter() const override { return 2.0 * (w_ + h_); }

    void bounding_box(double *ox, double *oy,
                      double *ow, double *oh) const override
    {
        *ox = x_;
        *oy = y_;
        *ow = w_;
        *oh = h_;
    }

    void scale(double factor, double *inout_area) override
    {
        w_ *= factor;
        h_ *= factor;
        *inout_area = area();
    }

    int get_label(char *buf, size_t len, size_t *out) const override
    {
        size_t n = std::strlen(label_);
        if (len < n + 1)
            return -1;
        std::memcpy(buf, label_, n + 1);
        if (out)
            *out = n;
        return 0;
    }

    void set_label(const char *l) override
    {
        std::strncpy(label_, l, sizeof(label_) - 1);
        label_[sizeof(label_) - 1] = '\0';
    }
};

/* ══════════════════════════════════════════════════════════════
 * Concrete: LabeledCircle  (IShape + ILabeled — multiple inheritance)
 * ══════════════════════════════════════════════════════════════ */

class LabeledCircle : public IShape, public ILabeled
{
    double cx_, cy_, r_;
    char label_[64];

public:
    LabeledCircle(const char *lbl, double cx, double cy, double r)
        : cx_(cx), cy_(cy), r_(r)
    {
        std::strncpy(label_, lbl, sizeof(label_) - 1);
        label_[sizeof(label_) - 1] = '\0';
    }

    double area() const override { return M_PI * r_ * r_; }
    double perimeter() const override { return 2.0 * M_PI * r_; }

    void bounding_box(double *ox, double *oy,
                      double *ow, double *oh) const override
    {
        *ox = cx_ - r_;
        *oy = cy_ - r_;
        *ow = 2.0 * r_;
        *oh = 2.0 * r_;
    }

    void scale(double factor, double *inout_area) override
    {
        r_ *= std::abs(factor);
        *inout_area = area();
    }

    int get_label(char *buf, size_t len, size_t *out) const override
    {
        size_t n = std::strlen(label_);
        if (len < n + 1)
            return -1;
        std::memcpy(buf, label_, n + 1);
        if (out)
            *out = n;
        return 0;
    }

    void set_label(const char *l) override
    {
        std::strncpy(label_, l, sizeof(label_) - 1);
        label_[sizeof(label_) - 1] = '\0';
    }
};

/* ══════════════════════════════════════════════════════════════
 * extern "C" — Factory + introspection only.  NO trampolines.
 * ══════════════════════════════════════════════════════════════ */

extern "C"
{

    /*
     * Handle returned by factory functions.
     *
     * `shape`   = IShape*   (primary base, also the allocation address)
     * `labeled` = ILabeled* (secondary base, pointer-adjusted by +8)
     *
     * Because IShape is the first (primary) base, `shape` equals the
     * raw `new`'d address.  Calling the deleting destructor through
     * the primary vtable (slot 1) will correctly `delete` the object.
     */
    struct CppDualHandle
    {
        void *shape;
        void *labeled;
    };

    /* ── Factories ──────────────────────────────────────────────── */

    CppDualHandle cpp_vt_create_rect(const char *label,
                                     double x, double y,
                                     double w, double height)
    {
        auto *obj = new LabeledRect(label, x, y, w, height);
        CppDualHandle hdl;
        hdl.shape = static_cast<IShape *>(obj);
        hdl.labeled = static_cast<ILabeled *>(obj);
        return hdl;
    }

    CppDualHandle cpp_vt_create_circle(const char *label,
                                       double cx, double cy,
                                       double r)
    {
        auto *obj = new LabeledCircle(label, cx, cy, r);
        CppDualHandle hdl;
        hdl.shape = static_cast<IShape *>(obj);
        hdl.labeled = static_cast<ILabeled *>(obj);
        return hdl;
    }

    /* ── Introspection helpers (for Rust test verification) ─────── */

    /*
     * Read a vtable function-pointer slot from an interface pointer.
     * `interface_ptr` is the address of the sub-object (where vptr
     * lives).  Returns the raw function pointer at `slot_index`.
     */
    void *cpp_vt_read_slot(const void *interface_ptr, int32_t slot_index)
    {
        void **vptr = *(void ***)interface_ptr;
        return vptr[slot_index];
    }

    /*
     * Read the offset-to-top metadata from the vtable.
     * This is at vptr[-2] (two pointer-sized entries before the
     * function-pointer array).
     *
     * For the primary base, offset_to_top == 0.
     * For a secondary base at offset N, offset_to_top == -N.
     */
    ptrdiff_t cpp_vt_offset_to_top(const void *interface_ptr)
    {
        ptrdiff_t *entries = *(ptrdiff_t **)interface_ptr;
        return entries[-2];
    }

    /* Return sizeof for the concrete types. */
    size_t cpp_vt_sizeof_rect(void) { return sizeof(LabeledRect); }
    size_t cpp_vt_sizeof_circle(void) { return sizeof(LabeledCircle); }

    /* Return the offset of the ILabeled sub-object within the
     * complete object (should be sizeof(void*) = 8 on 64-bit). */
    size_t cpp_vt_ilabeled_offset_in_rect(void)
    {
        LabeledRect *r = reinterpret_cast<LabeledRect *>(0x1000);
        auto *base = static_cast<ILabeled *>(r);
        return reinterpret_cast<size_t>(base) - 0x1000;
    }

    size_t cpp_vt_ilabeled_offset_in_circle(void)
    {
        LabeledCircle *c = reinterpret_cast<LabeledCircle *>(0x1000);
        auto *base = static_cast<ILabeled *>(c);
        return reinterpret_cast<size_t>(base) - 0x1000;
    }

} /* extern "C" */
