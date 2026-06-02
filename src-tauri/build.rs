fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "macos" {
        let out_dir = std::env::var("OUT_DIR").unwrap();

        // Resolve swiftc path once via xcrun — used for both compilation and lib discovery
        let xcrun_output = std::process::Command::new("xcrun")
            .args(["--find", "swiftc"])
            .output()
            .expect("Failed to execute xcrun to find swiftc");
        let swiftc_path = String::from_utf8(xcrun_output.stdout).unwrap();
        let swiftc = swiftc_path.trim();

        let status = std::process::Command::new(swiftc)
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

        println!("cargo:rustc-link-search=native={}", out_dir);
        println!("cargo:rustc-link-lib=static=MediaVolumeHelper");

        let swift_lib_path = std::path::Path::new(swiftc)
            .parent() // bin
            .and_then(|p| p.parent()) // usr
            .map(|p| p.join("lib/swift/macosx"))
            .expect("Failed to resolve Swift library path");

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
