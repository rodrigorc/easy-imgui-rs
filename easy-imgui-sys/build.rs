use std::env;
use std::path::PathBuf;
use xshell::Shell;

fn main() {
    //simple_logger::SimpleLogger::new().init().unwrap();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap();

    // imgui_ori is a sumbodule of the upstream imgui repository, and as such does not have a
    // proper imconfig.h for this projects. Changing that file in the submodule is inconvenient
    // because it would require me to maintain a fork just for that.
    // Adding an imconfig.h file outside of the source dir could break compilation of third-party
    // imgui modules, those that use $DEP_IMGUI_THIRD_PARTY.
    // It is safer to copy the source files into $OUT_DIR and write a new imconfig.h file there and
    // set $DEP_IMGUI_THIRD_PARTY to point to that.

    let sh = Shell::new().unwrap();
    let imgui_ori = manifest_dir.join("imgui");
    let imgui_src = out_path.join("imgui_src");
    let imgui_misc_ft = imgui_src.join("misc/freetype");

    sh.remove_path(&imgui_src).unwrap();
    sh.create_dir(&imgui_src).unwrap();
    sh.create_dir(&imgui_misc_ft).unwrap();

    for ori in [
        "imgui.h",
        "imgui_internal.h",
        "imstb_textedit.h",
        "imstb_rectpack.h",
        "imstb_truetype.h",
        "imgui.cpp",
        "imgui_widgets.cpp",
        "imgui_draw.cpp",
        "imgui_tables.cpp",
        "imgui_demo.cpp",
    ] {
        let src = imgui_ori.join(ori);
        sh.copy_file(&src, &imgui_src).unwrap();
        println!("cargo:rerun-if-changed={}", src.display());
    }
    for ori in ["imgui_freetype.cpp", "imgui_freetype.h"] {
        let src = imgui_ori.join("misc/freetype").join(ori);
        sh.copy_file(&src, &imgui_misc_ft).unwrap();
        println!("cargo:rerun-if-changed={}", src.display());
    }
    sh.write_file(
        imgui_src.join("imconfig.h"),
        r"
// This only works on windows, the arboard crate has better cross-support
#define IMGUI_DISABLE_WIN32_DEFAULT_CLIPBOARD_FUNCTIONS

// Only use the latest non-obsolete functions
#define IMGUI_DISABLE_OBSOLETE_FUNCTIONS
#define IMGUI_DISABLE_OBSOLETE_KEYIO
// A Rust char is 32-bits, just do that
#define IMGUI_USE_WCHAR32

// Try to play thread-safe-ish. The variable definition is in wrapper.cpp
struct ImGuiContext;
extern thread_local ImGuiContext* MyImGuiTLS;
#define GImGui MyImGuiTLS
        ",
    )
    .unwrap();

    println!("cargo:THIRD_PARTY={}", imgui_src.display());

    println!("cargo:rerun-if-changed=wrapper.cpp");

    println!(
        "cargo:rerun-if-changed={}/imgui.cpp",
        imgui_ori.to_string_lossy()
    );
    println!(
        "cargo:rerun-if-changed={}/imgui.h",
        imgui_ori.to_string_lossy()
    );

    let freetype = if cfg!(feature = "freetype") {
        Some(pkg_config::probe_library("freetype2").unwrap())
    } else {
        None
    };

    let mut bindings = bindgen::Builder::default();
    // Edition 2024!
    bindings = bindings
        .rust_target(bindgen::RustTarget::stable(85, 0).unwrap())
        .rust_edition(bindgen::RustEdition::Edition2024)
        .wrap_unsafe_ops(true);

    bindings = bindings
        .clang_args(["-I", &imgui_src.to_string_lossy()])
        .clang_args(["-x", "c++"])
        .clang_args(["-std=c++20"])
        .clang_args(["-D", "IMGUI_DISABLE_SSE"]) // that is only for inline functions
        .header(imgui_src.join("imgui.h").to_string_lossy())
        .header(imgui_src.join("imgui_internal.h").to_string_lossy())
        .allowlist_file(".*[/\\\\]imgui.h")
        // many people use the internals, so better to expose those, just do not use them lightly
        .allowlist_file(".*[/\\\\]imgui_internal.h")
        .prepend_enum_name(false)
        .bitfield_enum(".*Flags_")
        .newtype_enum(".*");

    if target_env == "msvc" {
        /* MSVC compilers have a weird ABI for C++. The only difference that affects us is a
         * function that returns a small C++ type, ie. a non-aggregate type, with sizeof <= 8.
         * In Dear ImGui, AFAIK, the only such type in the public API is ImVec2, that is 8 bytes
         * long and has a default constructor.
         * To solve this issue there are a few steps:
         *  * Blocklist in the bindings all the offending functions, even those unused, just in
         *    case.
         *  * Write and bind an equivalent C POD type for every needed non-POD type.
         *  * Write and bind C wrappers for the needed functions.
         *
         *  Due to a quirk in the handling of bindgen namespaces, we can blocklist `ImGui::Foo` and
         *  then write a wrapper `ImGui_Foo` and the Rust code won't even notice.
         */
        println!("cargo:rerun-if-changed=msvc_blocklist.txt");
        println!("cargo:rerun-if-changed=hack_msvc.cpp");
        for line in std::fs::read_to_string("msvc_blocklist.txt")
            .unwrap()
            .lines()
        {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            bindings = bindings.blocklist_function(line);
        }
        bindings = bindings
            .header("hack_msvc.cpp")
            .allowlist_file("hack_msvc.cpp");
    }
    if let Some(freetype) = &freetype {
        bindings = bindings.clang_arg("-DIMGUI_ENABLE_FREETYPE=1");
        for include in &freetype.include_paths {
            bindings = bindings.clang_args(["-I", &include.display().to_string()]);
        }
    }
    let bindings = bindings.generate().expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    let mut build = cc::Build::new();
    if target_arch != "wasm32" {
        build.cpp(true).std("c++20");
    }
    build.include(&imgui_src);
    build.file("wrapper.cpp");
    if let Some(freetype) = &freetype {
        build.define("IMGUI_ENABLE_FREETYPE", "1");
        for include in &freetype.include_paths {
            build.include(include.display().to_string());
        }
    }
    build.compile("dear_imgui");
}
