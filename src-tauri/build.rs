fn main() {
    cc::Build::new()
        .file("../native/bridge.m")
        .flag("-fobjc-arc")
        .flag("-mmacosx-version-min=13.0")
        .compile("qyj_bridge");
    for framework in ["AppKit", "Security", "CoreServices", "ServiceManagement"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
    println!("cargo:rerun-if-changed=../native/bridge.m");
    tauri_build::build();
}
