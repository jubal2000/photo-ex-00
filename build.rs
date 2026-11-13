fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "ios" {
        cc::Build::new()
            .file("src/platform/ios_file_picker.m")
            .file("src/platform/ios_apple_auth.m")
            .flag("-fobjc-arc")
            .compile("ios_native_helpers");
        println!("cargo:rerun-if-changed=src/platform/ios_file_picker.m");
        println!("cargo:rerun-if-changed=src/platform/ios_apple_auth.m");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=UIKit");
        println!("cargo:rustc-link-lib=framework=WebKit");
        println!("cargo:rustc-link-lib=framework=AuthenticationServices");
    }
}
