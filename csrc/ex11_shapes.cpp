/*
 * csrc/ex11_shapes.cpp
 *
 * Pre-provided C++ code for Exercise 11 (Vtable pattern).
 *
 * These functions receive an array of `CShape` structs and dispatch
 * through the vtable function pointers.  The function pointers
 * themselves are implemented in Rust by the student.
 */

#include <cstddef>
#include <cstdint>

extern "C" {

struct ShapeVTable {
    double (*area)(const void *data);
    double (*perimeter)(const void *data);
    void   (*destroy)(void *data);
};

struct CShape {
    void              *data;
    const ShapeVTable *vtable;
};

/* Sum the areas of all shapes by dispatching through each vtable. */
double cpp_total_area(const CShape *shapes, size_t count) {
    double total = 0.0;
    for (size_t i = 0; i < count; ++i) {
        if (shapes[i].vtable && shapes[i].vtable->area) {
            total += shapes[i].vtable->area(shapes[i].data);
        }
    }
    return total;
}

/* Destroy every shape via its vtable destroy function. */
void cpp_destroy_shapes(CShape *shapes, size_t count) {
    for (size_t i = 0; i < count; ++i) {
        if (shapes[i].vtable && shapes[i].vtable->destroy) {
            shapes[i].vtable->destroy(shapes[i].data);
        }
    }
}

} /* extern "C" */
