use std::env;
use std::path::PathBuf;

fn main() {
    tauri_build::build();
    
    // Only build Swift bridge on macOS
    #[cfg(target_os = "macos")]
    {
        build_swift_bridge();
    }
}

#[cfg(target_os = "macos")]
fn build_swift_bridge() {
    println!("cargo:rerun-if-changed=src/swift/FluidAudio.swift");
    
    // Use the library built by Swift Package Manager
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let lib_dir = manifest_dir.join("lib");
    
    // Check if library exists
    if !lib_dir.join("libFluidAudioBridge.dylib").exists() {
        eprintln!("FluidAudio library not found. Run `swift build -c release` first");
        panic!("FluidAudio library not found at {:?}", lib_dir.join("libFluidAudioBridge.dylib"));
    }
    
    // Link to the pre-built library
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=FluidAudioBridge");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=Accelerate");
    
    // Set rpath for different scenarios:
    // 1. Development: lib directory next to target/
    // 2. Production bundle: Frameworks directory inside .app bundle
    // @executable_path/../Frameworks is the standard location for bundled dylibs in macOS apps
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
    // Also keep the dev path for running from target/debug or target/release directly
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
}
