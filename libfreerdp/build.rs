fn main() {
    let cfg = pkg_config::probe_library("freerdp3").unwrap();

    println!("cargo:rerun-if-changed=src/trampolines.c");
    cc::Build::new()
        .file("src/trampolines.c")
        .includes(cfg.include_paths)
        .flag("-Wno-deprecated-declarations")
        .compile("sztsc-libfreerdp");
}
