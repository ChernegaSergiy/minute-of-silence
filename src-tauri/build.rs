fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "macos" {
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let status = std::process::Command::new("swiftc")
            .args([
                "-parse-as-library",
                "-g",
                "-O",
                "-emit-library",
                "-static",
                "-o",
                &format!("{}/libMediaVolumeHelper.a", out_dir),
                "src/platform/macos/MediaVolumeHelper.swift",
            ])
            .status()
            .unwrap();

        if !status.success() {
            panic!("Swift compilation failed");
        }

        // Link the compiled static library
        println!("cargo:rustc-link-search=native={}", out_dir);
        println!("cargo:rustc-link-lib=static=MediaVolumeHelper");

        // Resolve Swift library path dynamically
        let xcrun_output = std::process::Command::new("xcrun")
            .args(["--find", "swiftc"])
            .output()
            .expect("Failed to execute xcrun to find swiftc");

        let swiftc_path = String::from_utf8(xcrun_output.stdout).unwrap();
        let swift_lib_path = std::path::Path::new(swiftc_path.trim())
            .parent() // -> usr/bin
            .and_then(|p| p.parent()) // -> usr
            .map(|p| p.join("lib/swift/macosx"))
            .expect("Failed to resolve Swift library path");

        // Link Swift runtime libraries
        println!("cargo:rustc-link-search=native=/usr/lib/swift");
        println!(
            "cargo:rustc-link-search=native={}",
            swift_lib_path.display()
        );
        println!("cargo:rustc-link-lib=dylib=swiftCore");
        println!("cargo:rustc-link-lib=dylib=swiftAppKit");
    }

    tauri_build::build();
}
