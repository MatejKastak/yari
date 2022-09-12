extern crate bindgen;

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug)]
struct IgnoreMacros(HashSet<String>);

impl bindgen::callbacks::ParseCallbacks for IgnoreMacros {
    fn will_parse_macro(&self, name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        if self.0.contains(name) {
            bindgen::callbacks::MacroParsingBehavior::Ignore
        } else {
            bindgen::callbacks::MacroParsingBehavior::Default
        }
    }
}

fn main() {
    let ignored_macros = IgnoreMacros(
        vec![
            "FP_INFINITE".into(),
            "FP_NAN".into(),
            "FP_NORMAL".into(),
            "FP_SUBNORMAL".into(),
            "FP_ZERO".into(),
            "IPPORT_RESERVED".into(),
        ]
        .into_iter()
        .collect(),
    );

    println!("cargo:rustc-link-lib=static=yara");
    println!("cargo:rustc-link-lib=crypto");
    println!("cargo:rustc-link-lib=magic");
    println!("cargo:rustc-link-lib=jansson");
    println!("cargo:rustc-link-lib=z");

    if let Some(lib_dirs) = std::env::var_os("YARI_LIB_DIRS") {
        for lib in std::env::split_paths(&lib_dirs) {
            println!(
                "cargo:rustc-link-search={}",
                lib.to_str().expect("Cannot process YARI_LIBS_DIR")
            );
        }
    }

    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    let yara_repo_root = option_env!("YARI_YARA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate_root.join("./yara"));

    let libyara_dir = yara_repo_root.join("libyara/.libs/");
    let libyara_includes = yara_repo_root.join("libyara/include/");

    println!(
        "cargo:rustc-link-search={}",
        libyara_dir.to_str().expect("cannot find YARA libraries")
    );

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");
    println!(
        "cargo:rerun-if-changed={}",
        yara_repo_root
            .to_str()
            .expect("YARA repo is not valid path")
    );

    let use_bundled_bindings = option_env!("YARI_USE_BUNDLED_BINDINGS");
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = PathBuf::from(out_dir).join("bindings.rs");
    if use_bundled_bindings.is_some() {
        let binding_file = "bindings-unix.rs";
        fs::copy(PathBuf::from("bindings").join(binding_file), out_path)
            .expect("Could not copy bindings to output directory");
    } else if let Some(bindings_file) = option_env!("YARI_BINDINGS_FILE") {
        let bindings_file = Path::new(bindings_file);
        fs::copy(bindings_file, out_path).expect("Could not copy bindings to output directory");
    } else {
        let bindings = bindgen::Builder::default()
            .header("wrapper.h")
            .allowlist_var("YR_.*")
            .allowlist_var("ERROR_.*")
            .allowlist_var("META_TYPE_.*")
            .allowlist_var("OBJECT_TYPE_.*")
            .allowlist_function("yr_.*")
            .allowlist_type("YR_.*")
            .blocklist_item("_SIZED_STRING")
            .blocklist_item("SIZED_STRING")
            .clang_arg(format!(
                "-I{}",
                libyara_includes
                    .to_str()
                    .expect("invalid YARA includes path")
            ))
            .rustfmt_bindings(true)
            .derive_debug(true)
            .derive_default(true)
            .parse_callbacks(Box::new(ignored_macros))
            .generate()
            .expect("Unable to generate bindings");

        // Write the bindings to the $OUT_DIR/bindings.rs file.
        bindings
            .write_to_file(out_path)
            .expect("Couldn't write bindings!");
    }
}
