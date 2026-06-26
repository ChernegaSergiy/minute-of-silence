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

        let sdk = {
            let output = std::process::Command::new("xcrun")
                .args(["--sdk", "macosx", "--show-sdk-path"])
                .output()
                .expect("xcrun failed");
            String::from_utf8(output.stdout).unwrap().trim().to_string()
        };

        let m_sources = [
            "src/platform/macos/theme.m",
            "src/platform/macos/now_playing.m",
        ];

        for (i, src) in m_sources.iter().enumerate() {
            let obj = format!("{out_dir}/obj{i}.o");
            let status = std::process::Command::new("clang")
                .args([
                    "-target",
                    clang_target,
                    "-fobjc-arc",
                    "-O2",
                    "-isysroot",
                    &sdk,
                    "-c",
                    "-o",
                    &obj,
                    src,
                ])
                .status()
                .unwrap();

            if !status.success() {
                panic!("Objective-C compilation failed for {}", src);
            }
        }

        let ar_status = std::process::Command::new("ar")
            .arg("rcs")
            .arg(format!("{out_dir}/libmacos_theme.a"))
            .arg(format!("{out_dir}/obj0.o"))
            .arg(format!("{out_dir}/obj1.o"))
            .status()
            .unwrap();

        if !ar_status.success() {
            panic!("ar failed");
        }

        println!("cargo:rustc-link-search=native={}", out_dir);
        println!("cargo:rustc-link-lib=static=macos_theme");
        println!("cargo:rustc-link-lib=objc");
        println!("cargo:rustc-link-arg=-Wl,-framework,Foundation");
        println!("cargo:rustc-link-arg=-Wl,-framework,AppKit");
        println!("cargo:rustc-link-arg=-Wl,-framework,CoreAudio");
        println!("cargo:rustc-link-arg=-Wl,-F,/System/Library/PrivateFrameworks");
        println!("cargo:rustc-link-arg=-Wl,-framework,MediaRemote");
    }

    tauri_build::build();
}
