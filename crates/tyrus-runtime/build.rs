fn main() {
    println!("cargo:rerun-if-changed=src/platform/stdio.c");
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|target| target != "unknown") {
        cc::Build::new()
            .file("src/platform/stdio.c")
            .warnings(true)
            .compile("tyrus_stdio");
    }
}
