fn main() {
    build_locales();
}

#[cfg(feature = "tr")]
fn build_locales() {
    let output_dir = std::env::var("OUT_DIR").unwrap();
    let src = std::path::PathBuf::from(&output_dir).join("locale/translators.rs");
    include_po::generate_locales_from_dir("locales", src).unwrap();
}

#[cfg(not(feature = "tr"))]
fn build_locales() {}
