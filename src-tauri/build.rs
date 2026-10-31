//! Tauri's build step, plus the Windows application manifest for every
//! executable of this crate.
//!
//! Tauri links its manifest (Common Controls v6) into the app binary only.
//! Test binaries then fail to start on Windows with `STATUS_ENTRYPOINT_NOT_FOUND`
//! (0xc0000139), because `tauri` imports functions that only Common Controls v6
//! has. Embedding the manifest through the linker covers tests too.

use std::env;
use std::path::Path;

const WINDOWS_MANIFEST_FILE: &str = "windows-app-manifest.xml";

fn main() {
    let mut windows = tauri_build::WindowsAttributes::new();
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        windows = tauri_build::WindowsAttributes::new_without_app_manifest();
        embed_manifest(&Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join(WINDOWS_MANIFEST_FILE));
    }
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows)).expect("tauri build step");
}

fn embed_manifest(manifest: &Path) {
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    // A manifest the linker can't use should fail the build, not the app at runtime.
    println!("cargo:rustc-link-arg=/WX");
}
