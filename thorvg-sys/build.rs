use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let thorvg_src = manifest_dir.join("thorvg");

    if cfg!(feature = "vendored") {
        build_vendored(&thorvg_src, &out_dir);
    } else {
        link_system();
    }

    generate_bindings(&thorvg_src, &out_dir);
}

/// Build thorvg from the vendored source using meson + ninja.
fn build_vendored(thorvg_src: &Path, out_dir: &Path) {
    let build_dir = out_dir.join("thorvg-build");
    let install_dir = out_dir.join("thorvg-install");

    // Check for meson and ninja
    check_tool(
        "meson",
        "meson is required to build thorvg. Install it with: pip install meson",
    );
    check_tool(
        "ninja",
        "ninja is required to build thorvg. Install it with your package manager.",
    );

    // Configure with meson (only if not already configured)
    if !build_dir.join("build.ninja").exists() {
        let status = Command::new("meson")
            .arg("setup")
            .arg(&build_dir)
            .arg(thorvg_src)
            .arg(format!("--prefix={}", install_dir.display()))
            .arg("--default-library=static")
            .arg("--buildtype=release")
            .arg("-Dbindings=capi")
            .arg("-Dloaders=all")
            .arg("-Dthreads=true")
            .arg("-Dstatic=true")
            // Disable tools and tests for faster builds
            .arg("-Dtools=")
            .arg("-Dtests=false")
            .arg("-Dlog=false")
            .status()
            .expect("Failed to run meson setup");

        assert!(status.success(), "meson setup failed");
    }

    // Build with ninja
    let status = Command::new("ninja")
        .arg("-C")
        .arg(&build_dir)
        .status()
        .expect("Failed to run ninja");

    assert!(status.success(), "ninja build failed");

    // Install to get clean lib/include layout
    let status = Command::new("meson")
        .arg("install")
        .arg("-C")
        .arg(&build_dir)
        .status()
        .expect("Failed to run meson install");

    assert!(status.success(), "meson install failed");

    // Find the static library
    // meson installs to lib/, lib64/, or lib/x86_64-linux-gnu/ depending on distro
    let lib_dir = find_lib_dir(&install_dir);

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=thorvg-1");

    // ThorVG depends on these system libraries
    let target = env::var("TARGET").unwrap();
    if target.contains("apple") || target.contains("freebsd") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else if target.contains("linux") || target.contains("gnu") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    // Link pthread for threading support
    println!("cargo:rustc-link-lib=dylib=pthread");

    // Link OpenMP (thorvg uses it for parallel rasterization)
    link_optional_dep("openmp");
    // Fallback: link gomp directly if pkg-config doesn't find openmp
    if pkg_config::probe_library("openmp").is_err() {
        println!("cargo:rustc-link-lib=dylib=gomp");
    }

    // Link optional loader dependencies (if available)
    link_optional_dep("libpng");
    link_optional_dep("libturbojpeg");
    link_optional_dep("libwebp");

    println!("cargo:rerun-if-changed=thorvg/src");
    println!("cargo:rerun-if-changed=thorvg/inc");
    println!("cargo:rerun-if-changed=thorvg/meson.build");
}

/// Link against a system-installed thorvg via pkg-config.
fn link_system() {
    pkg_config::Config::new()
        .atleast_version("1.0.0")
        .probe("thorvg")
        .expect(
            "Could not find system thorvg >= 1.0.0 via pkg-config. \
             Either install thorvg or enable the `vendored` feature.",
        );
}

/// Generate Rust bindings from the C API header.
fn generate_bindings(thorvg_src: &Path, out_dir: &Path) {
    let capi_header = thorvg_src
        .join("src")
        .join("bindings")
        .join("capi")
        .join("thorvg_capi.h");

    println!("cargo:rerun-if-changed={}", capi_header.display());

    let bindings = bindgen::Builder::default()
        .header(capi_header.to_string_lossy())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_function("tvg_.*")
        .allowlist_type("Tvg_.*")
        .allowlist_var("TVG_.*")
        // Generate Rust enums from C enums
        .rustified_enum("Tvg_Result")
        .rustified_enum("Tvg_Colorspace")
        .rustified_enum("Tvg_Engine_Option")
        .rustified_enum("Tvg_Mask_Method")
        .rustified_enum("Tvg_Blend_Method")
        .rustified_enum("Tvg_Type")
        .rustified_enum("Tvg_Stroke_Cap")
        .rustified_enum("Tvg_Stroke_Join")
        .rustified_enum("Tvg_Stroke_Fill")
        .rustified_enum("Tvg_Fill_Rule")
        .rustified_enum("Tvg_Text_Wrap")
        .rustified_enum("Tvg_Filter_Method")
        // no_std support: use core types instead of std
        .use_core()
        .layout_tests(true)
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

/// Check that a build tool is available on PATH.
fn check_tool(name: &str, help: &str) {
    let result = Command::new("which").arg(name).output();
    match result {
        Ok(output) if output.status.success() => {}
        _ => panic!("{name} not found. {help}"),
    }
}

/// Find the library directory under the install prefix.
fn find_lib_dir(install_dir: &Path) -> PathBuf {
    let candidates = [
        install_dir.join("lib"),
        install_dir.join("lib64"),
        install_dir.join("lib").join("x86_64-linux-gnu"),
    ];

    for dir in &candidates {
        if dir.join("libthorvg-1.a").exists() {
            return dir.clone();
        }
    }

    // Fall back to lib/
    candidates[0].clone()
}

/// Try to link an optional system dependency via pkg-config.
fn link_optional_dep(name: &str) {
    // We only print the link flags; if the dep is missing the loader just won't work
    // at runtime (thorvg handles this gracefully).
    if let Ok(lib) = pkg_config::probe_library(name) {
        for path in &lib.link_paths {
            println!("cargo:rustc-link-search=native={}", path.display());
        }
        for lib_name in &lib.libs {
            println!("cargo:rustc-link-lib=dylib={lib_name}");
        }
    }
}
