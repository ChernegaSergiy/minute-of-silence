fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "macos" {
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let target = std::env::var("TARGET").unwrap();

        // Map Rust target to Swift target
        let swift_target = if target == "x86_64-apple-darwin" {
            "x86_64-apple-macosx11.0"
        } else if target == "aarch64-apple-darwin" {
            "arm64-apple-macosx11.0"
        } else {
            &target
        };

        let swiftc = if let Ok(path) = std::env::var("SWIFTC") {
            path.trim().to_string()
        } else {
            let xcrun_output = std::process::Command::new("xcrun")
                .args(["--find", "swiftc"])
                .output()
                .expect("Failed to execute xcrun to find swiftc");
            String::from_utf8(xcrun_output.stdout)
                .unwrap()
                .trim()
                .to_string()
        };

        let sdk_output = std::process::Command::new("xcrun")
            .args(["--sdk", "macosx", "--show-sdk-path"])
            .output()
            .expect("Failed to execute xcrun to find SDK path");
        let sdk_path = String::from_utf8(sdk_output.stdout).unwrap();
        let sdk = sdk_path.trim();

        let status = std::process::Command::new(&swiftc)
            .args([
                "-target",
                swift_target,
                "-parse-as-library",
                "-g",
                "-O",
                "-emit-library",
                "-static",
                "-sdk",
                sdk,
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

        let swift_lib_path = std::path::Path::new(&swiftc)
            .parent()
            .and_then(|p| p.parent())
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
