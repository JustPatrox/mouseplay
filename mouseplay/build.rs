fn main() {
    let target = std::env::var("TARGET").expect("Cargo must provide TARGET");
    if target.contains("windows") {
        return;
    }

    println!("cargo:rerun-if-changed=bridge/chiaki_bridge.c");
    println!("cargo:rerun-if-changed=bridge/chiaki_bridge.h");
    println!("cargo:rerun-if-changed=../third-party/chiaki-ng/lib/include/chiaki/controller.h");

    cc::Build::new()
        .file("bridge/chiaki_bridge.c")
        .include("bridge")
        .include("../third-party/chiaki-ng/lib/include")
        .warnings(true)
        .compile("mouseplay_chiaki_bridge");
}
