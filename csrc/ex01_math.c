/*
 * csrc/ex01_math.c
 *
 * Pre-provided C functions for Exercise 01.
 * Students call these from Rust via `extern "C"` declarations.
 */

#include <math.h>

int c_add(int a, int b) {
    return a + b;
}

int c_multiply(int a, int b) {
    return a * b;
}

double c_distance(double x1, double y1, double x2, double y2) {
    double dx = x2 - x1;
    double dy = y2 - y1;
    return sqrt(dx * dx + dy * dy);
}

int c_abs(int x) {
    return x < 0 ? -x : x;
}
