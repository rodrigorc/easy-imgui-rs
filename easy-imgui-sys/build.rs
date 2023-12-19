use std::env;
use std::path::PathBuf;
use xshell::Shell;

fn main() {
    //simple_logger::SimpleLogger::new().init().unwrap();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    // imgui_ori is a sumbodule of the upstream imgui repository, and as such does not have a
    // proper imconfig.h for this projects. Changing that file in the submodule is inconvenient
    // because it would require me to maintain a fork just for that.
    // Adding an imconfig.h file outside of the source dir could break compilation of third-party
    // imgui modules, those that use $DEP_IMGUI_THIRD_PARTY.
    // It is safer to copy the source files into $OUT_DIR and write a new imconfig.h file there and
    // set %DEP_IMGUI_THIRD_PARTY to point to that.

    let sh = Shell::new().unwrap();
    let imgui_ori = if cfg!(feature="docking") {
        manifest_dir.join("imgui-docking")
    } else {
        manifest_dir.join("imgui")
    };
    let imgui_src = out_path.join("imgui_src");
    let imgui_misc_ft = imgui_src.join("misc/freetype");

    sh.remove_path(&imgui_src).unwrap();
    sh.create_dir(&imgui_src).unwrap();
    sh.create_dir(&imgui_misc_ft).unwrap();

    for ori in
        [
            "imgui.h", "imgui_internal.h", "imstb_textedit.h", "imstb_rectpack.h", "imstb_truetype.h",
            "imgui.cpp", "imgui_widgets.cpp", "imgui_draw.cpp", "imgui_tables.cpp", "imgui_demo.cpp"
        ]
    {
        sh.copy_file(imgui_ori.join(ori), &imgui_src).unwrap();
        println!("cargo:rerun-if-changed={}/{}", imgui_ori.display(), ori);
    }
    sh.copy_file(imgui_ori.join("misc/freetype/imgui_freetype.cpp"), &imgui_misc_ft).unwrap();
    sh.copy_file(imgui_ori.join("misc/freetype/imgui_freetype.h"), &imgui_misc_ft).unwrap();
    sh.write_file(imgui_src.join("imconfig.h"), r"
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
        ").unwrap();

    println!(
        "cargo:THIRD_PARTY={}",
        imgui_src.display()
    );

    println!("cargo:rerun-if-changed=wrapper.cpp");

    println!("cargo:rerun-if-changed={}/imgui.cpp", imgui_ori.to_string_lossy());
    println!("cargo:rerun-if-changed={}/imgui.h", imgui_ori.to_string_lossy());

    let freetype = if cfg!(feature = "freetype") {
        Some(pkg_config::probe_library("freetype2").unwrap())
    } else {
        None
    };

    let mut bindings = bindgen::Builder::default();
    bindings = bindings
        .clang_args(["-I", &imgui_src.to_string_lossy()])
        .clang_args(["-x", "c++"])
        .clang_args(["-D", "IMGUI_DISABLE_SSE"]) // that is only for inline functions
        .header(imgui_src.join("imgui.h").to_string_lossy())
        .header(imgui_src.join("imgui_internal.h").to_string_lossy())
        .allowlist_file(".*/imgui.h")
        // many people use the internals, so better to expose those, just do not use them lightly
        .allowlist_file(".*/imgui_internal.h")
        .prepend_enum_name(false)
        .bitfield_enum(".*Flags_")
        .newtype_enum(".*");

    if let Some(freetype) = &freetype {
        bindings = bindings.clang_arg("-DIMGUI_ENABLE_FREETYPE=1");
        for include in &freetype.include_paths {
            bindings = bindings.clang_args(["-I", &include.display().to_string()]);
        }
    }
    let bindings = bindings
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    let mut build = cc::Build::new();
    if target_arch != "wasm32" {
        build
            .cpp(true);
    }
    build
        .file("wrapper.cpp")
        .include(&imgui_src);
    if let Some(freetype) = &freetype {
        build.define("IMGUI_ENABLE_FREETYPE", "1");
        for include in &freetype.include_paths {
            build.include(&include.display().to_string());
        }
    }
    build
        .compile("dear_imgui");
}
