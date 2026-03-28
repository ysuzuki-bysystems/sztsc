fn main() {
    let cfg = pkg_config::probe_library("freerdp3").unwrap();

    println!("cargo:rerun-if-changed=src/trampolines.c");
    println!("cargo:rerun-if-changed=src/pubsub.c");
    cc::Build::new()
        .file("src/trampolines.c")
        .file("src/pubsub.c")
        .includes(&cfg.include_paths)
        .flag("-Wno-deprecated-declarations")
        .compile("sztsc-libfreerdp");
}
