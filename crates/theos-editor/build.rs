//! Build script: generates the Tauri context and, on Windows with MSVC, embeds the app manifest
//! (Common Controls v6) in every target. `tauri-build` links it into binaries only, and a test
//! harness without it exits with `STATUS_ENTRYPOINT_NOT_FOUND` (P11 §4.2).

use std::path::Path;

fn main() -> tauri_build::Result<()> {
    let windows_msvc = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    let attributes = if windows_msvc {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        tauri_build::Attributes::new()
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest())
    } else {
        tauri_build::Attributes::new()
    };
    tauri_build::try_build(attributes)
}
