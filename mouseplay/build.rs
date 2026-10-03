fn main() {
    let target = std::env::var("TARGET").expect("Cargo must provide TARGET");

    println!("cargo:rerun-if-changed=bridge/chiaki_bridge.c");
    println!("cargo:rerun-if-changed=bridge/chiaki_bridge.h");
    println!("cargo:rerun-if-changed=../third-party/chiaki-ng/lib/include/chiaki/controller.h");

    let mut build = cc::Build::new();
    build
        .file("bridge/chiaki_bridge.c")
        .include("bridge")
        .include("../third-party/chiaki-ng/lib/include")
        .warnings(true);

    if target.contains("apple-darwin") {
        let build_dir = std::env::var_os("MOUSEPLAY_CHIAKI_BUILD_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
                    .join("../target/chiaki-ng-arm64-shared")
            });
        let lib_dir = build_dir.join("lib");
        let generated_include = lib_dir.join("include");
        let dylib = lib_dir.join("libchiaki.dylib");
        if !dylib.is_file() {
            panic!(
                "libchiaki not found at {}. Build it with scripts/build-chiaki-macos.sh or set MOUSEPLAY_CHIAKI_BUILD_DIR",
                dylib.display()
            );
        }
        println!("cargo:rerun-if-env-changed=MOUSEPLAY_CHIAKI_BUILD_DIR");
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-lib=dylib=chiaki");
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        if let Some(profile_dir) = out_dir.ancestors().nth(3) {
            let deps_dir = profile_dir.join("deps");
            std::fs::create_dir_all(&deps_dir).expect("create Cargo deps directory");
            std::fs::copy(&dylib, deps_dir.join("libchiaki.dylib"))
                .expect("copy libchiaki next to Cargo test binaries");
        }
        build.include(generated_include);
        build.define("MOUSEPLAY_CHIAKI_LINKED", None);
    }

    build.compile("mouseplay_chiaki_bridge");
}
