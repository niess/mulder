fn main() {
    let c_includes = [
        "deps/gull/include",
        "deps/pumas/include",
        "deps/turtle/include",
        "deps/turtle/src",
    ];

    let c_headers = [
        "deps/gull/include/gull.h",
        "deps/pumas/include/pumas.h",
        "deps/turtle/include/turtle.h",
        "deps/turtle/src/turtle/client.h",
        "deps/turtle/src/turtle/error.h",
        "deps/turtle/src/turtle/io.h",
        "deps/turtle/src/turtle/list.h",
        "deps/turtle/src/turtle/map.h",
        "deps/turtle/src/turtle/projection.h",
        "deps/turtle/src/turtle/stack.h",
        "deps/turtle/src/turtle/stepper.h",
        "deps/turtle/src/deps/jsmn.h",
        "deps/turtle/src/deps/tinydir.h",
    ];

    let c_sources = [
        "deps/gull/src/gull.c",
        "deps/pumas/src/pumas.c",
        "deps/turtle/src/turtle/client.c",
        "deps/turtle/src/turtle/ecef.c",
        "deps/turtle/src/turtle/error.c",
        "deps/turtle/src/turtle/io.c",
        "deps/turtle/src/turtle/list.c",
        "deps/turtle/src/turtle/map.c",
        "deps/turtle/src/turtle/projection.c",
        "deps/turtle/src/turtle/stack.c",
        "deps/turtle/src/turtle/stepper.c",
        "deps/turtle/src/turtle/io/asc.c",
        "deps/turtle/src/turtle/io/grd.c",
        "deps/turtle/src/turtle/io/hgt.c",
        "deps/turtle/src/deps/jsmn.c",
        "deps/turtle/src/deps/tinydir.c",
    ];

    let cstd = if cfg!(windows) { "c11" } else { "c99" };
    cc::Build::new()
        .cpp(false)
        .std(cstd)
        .includes(c_includes)
        .files(c_sources)
        .define("TURTLE_NO_LD", None)
        .define("TURTLE_NO_PNG", None)
        .define("TURTLE_NO_TIFF", None)
        .warnings(false)
        .compile("c-libs");

    for path in c_headers.iter()
        .chain(c_sources.iter()) {
        println!("cargo:rerun-if-changed={}", path);
    }
}
