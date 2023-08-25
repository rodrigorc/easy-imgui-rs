use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    println!(
        "cargo:THIRD_PARTY={}",
        manifest_dir.join("imgui").display()
    );

    println!("cargo:rerun-if-changed=wrapper.cpp");

    let freetype = if cfg!(feature = "freetype") {
        Some(pkg_config::probe_library("freetype2").unwrap())
    } else {
        None
    };

    let mut bindings = bindgen::Builder::default()
        .clang_args(["-I", "imgui"])
        .clang_args(["-x", "c++"])
        .header("imgui/imgui.h")
        .allowlist_file(".*/imgui.h")
        .prepend_enum_name(false)
        .bitfield_enum(".*Flags_")
        .newtype_enum(".*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks));

    if let Some(freetype) = &freetype {
        bindings = bindings.clang_arg("-DIMGUI_ENABLE_FREETYPE=1");
        for include in &freetype.include_paths {
            bindings = bindings.clang_args(["-I", &include.display().to_string()]);
        }
    }
    let bindings = bindings
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("wrapper.cpp")
        .include("imgui");
    if let Some(freetype) = &freetype {
        build.define("IMGUI_ENABLE_FREETYPE", "1");
        for include in &freetype.include_paths {
            build.include(&include.display().to_string());
        }
    }
    build
        .compile("dear_imgui");
}