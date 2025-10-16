use std::env;
use std::path::{Path, PathBuf};
use xshell::Shell;

fn main() {
    dbg!(std::env::vars());

    //simple_logger::SimpleLogger::new().init().unwrap();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

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
    let imgui_backends = imgui_src.join("backends");

    sh.remove_path(&imgui_src).unwrap();
    sh.create_dir(&imgui_src).unwrap();
    sh.create_dir(&imgui_misc_ft).unwrap();
    sh.create_dir(&imgui_backends).unwrap();

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

    let mut backend_files = Vec::new();

    if cfg!(feature = "backend-opengl3") {
        backend_files.extend_from_slice(&[
            "imgui_impl_opengl3.cpp",
            "imgui_impl_opengl3.h",
            "imgui_impl_opengl3_loader.h",
        ]);
    }
    if cfg!(feature = "backend-sdl3") {
        backend_files.extend_from_slice(&["imgui_impl_sdl3.cpp", "imgui_impl_sdl3.h"]);
    }

    for ori in backend_files {
        let src = imgui_ori.join("backends").join(ori);
        sh.copy_file(&src, &imgui_backends).unwrap();
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

    let sdl3_src_dir = if cfg!(feature = "backend-sdl3") {
        // If sdl3-sys uses a copy of the SDL3 source, we'll use the same.
        env::var("DEP_SDL3_ROOT")
            .ok()
            .map(|root| Path::new(&root).join("include"))
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

    if cfg!(feature = "backend-opengl3") {
        bindings = bindings
            .header(
                imgui_backends
                    .join("imgui_impl_opengl3.h")
                    .to_string_lossy(),
            )
            .allowlist_file(".*[/\\\\]imgui_impl_opengl3.h");
    }
    if cfg!(feature = "backend-sdl3") {
        bindings = bindings
            .header(imgui_backends.join("imgui_impl_sdl3.h").to_string_lossy())
            .blocklist_type("SDL.*")
            .allowlist_file(".*[/\\\\]imgui_impl_sdl3.h");
        if let Some(sdl3_src_dir) = &sdl3_src_dir {
            bindings = bindings.clang_args(["-I", &sdl3_src_dir.to_string_lossy()]);
        }
    }

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
    if target_arch == "wasm32" {
        build.define("IMGUI_DISABLE_DEFAULT_SHELL_FUNCTIONS", "1");
    } else {
        build.cpp(true).std("c++20");
        if target_os == "windows" {
            // ImGui uses ShellExecute, with MSVC it uses the pragma lib trick, but with GNU it won't work.
            // No harm in setting this twice.
            println!("cargo:rustc-link-lib=shell32");
        }
    }
    build.include(&imgui_src);
    build.file("wrapper.cpp");
    build.file(imgui_src.join("imgui.cpp"));
    build.file(imgui_src.join("imgui_widgets.cpp"));
    build.file(imgui_src.join("imgui_draw.cpp"));
    build.file(imgui_src.join("imgui_tables.cpp"));
    build.file(imgui_src.join("imgui_demo.cpp"));
    if let Some(freetype) = &freetype {
        build.file(imgui_misc_ft.join("imgui_freetype.cpp"));
        build.define("IMGUI_ENABLE_FREETYPE", "1");
        for include in &freetype.include_paths {
            build.include(include.display().to_string());
        }
    }
    if cfg!(feature = "backend-opengl3") {
        build.file(imgui_backends.join("imgui_impl_opengl3.cpp"));
    }
    if cfg!(feature = "backend-sdl3") {
        build.file(imgui_backends.join("imgui_impl_sdl3.cpp"));
        if let Some(sdl3_src_dir) = &sdl3_src_dir {
            build.include(sdl3_src_dir);
        }
    }
    build.compile("dear_imgui");
}
