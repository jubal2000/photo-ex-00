fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "ios" {
        cc::Build::new()
            .file("src/platform/ios_file_picker.m")
            .flag("-fobjc-arc")
            .compile("ios_file_picker");
        println!("cargo:rerun-if-changed=src/platform/ios_file_picker.m");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=UIKit");
        println!("cargo:rustc-link-lib=framework=WebKit");
    }
}
