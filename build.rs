fn main() {
    // Compile the C helpers that some exercises call from Rust.
    cc::Build::new()
        .file("csrc/ex01_math.c")
        .file("csrc/ex07_c_callbacks.c")
        .file("csrc/ex12_blocking.c")
        .file("csrc/ex17_cdb.c")
        .file("csrc/ex18_closures.c")
        .file("csrc/ex24_session.c")
        .include("include")
        .compile("c_exercises");

    // Compile the C++ helpers.
    cc::Build::new()
        .cpp(true)
        .file("csrc/ex11_shapes.cpp")
        .file("csrc/ex25_cpp_virtual.cpp")
        .file("csrc/ex26_cpp_exceptions.cpp")
        .file("csrc/ex27_cpp_stl.cpp")
        .include("include")
        .compile("cpp_exercises");

    println!("cargo:rerun-if-changed=csrc/");
    println!("cargo:rerun-if-changed=include/");

    // Ensure C++ standard library is linked (needed for integration tests)
    println!("cargo:rustc-link-lib=stdc++");
}
