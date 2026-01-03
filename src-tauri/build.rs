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
    
    // Set rpath so the binary can find the dylib at runtime
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../lib");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
}
