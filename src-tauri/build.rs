fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "macos" {
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let target = std::env::var("TARGET").unwrap();

        let clang_target = if target == "x86_64-apple-darwin" {
            "x86_64-apple-macosx13.0"
        } else if target == "aarch64-apple-darwin" {
            "arm64-apple-macosx13.0"
        } else {
            &target
        };

        let sdk_output = std::process::Command::new("xcrun")
            .args(["--sdk", "macosx", "--show-sdk-path"])
            .output()
            .expect("Failed to execute xcrun to find SDK path");
        let sdk_path = String::from_utf8(sdk_output.stdout).unwrap();
        let sdk = sdk_path.trim();

        let obj_status = std::process::Command::new("clang")
            .args([
                "-target",
                clang_target,
                "-fobjc-arc",
                "-O2",
                "-isysroot",
                sdk,
                "-c",
                "-o",
                &format!("{}/MediaVolumeHelper.o", out_dir),
                "src/platform/macos/MediaVolumeHelper.m",
            ])
            .status()
            .unwrap();

        if !obj_status.success() {
            panic!("Objective-C compilation failed");
        }

        let ar_status = std::process::Command::new("ar")
            .args([
                "rcs",
                &format!("{}/libMediaVolumeHelper.a", out_dir),
                &format!("{}/MediaVolumeHelper.o", out_dir),
            ])
            .status()
            .unwrap();

        if !ar_status.success() {
            panic!("ar failed to create static library");
        }

        println!("cargo:rustc-link-search=native={}", out_dir);
        println!("cargo:rustc-link-lib=static=MediaVolumeHelper");
        println!("cargo:rustc-link-lib=objc");
        println!("cargo:rustc-link-arg=-Wl,-framework,Foundation");
        println!("cargo:rustc-link-arg=-Wl,-framework,AppKit");
        println!("cargo:rustc-link-arg=-Wl,-framework,CoreAudio");
    }

    tauri_build::build();
}
