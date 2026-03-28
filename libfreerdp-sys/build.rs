use std::{env, path::PathBuf};

fn main() {
    let cfg = pkg_config::probe_library("freerdp-client3").unwrap();
    for lib in cfg.libs {
        println!("cargo:rustc-link-lib={lib}");
    }

    println!("cargo:rerun-if-changed=wrapper.h");
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_args(
            cfg.include_paths
                .iter()
                .map(|v| format!("-I{}", v.to_string_lossy()))
                .collect::<Vec<_>>(),
        )
        .clang_arg("-Wno-deprecated-declarations")
        .allowlist_var("PTR_FLAGS_.*|.*_CHANNEL_NAME|RDP_CLIENT_INTERFACE_VERSION")
        .allowlist_type("DrdynvcClientContext|DispClientContext")
        //.allowlist_file("")
        .allowlist_function("(freerdp|gdi)_.*|WaitForMultipleObjects|CreateFileDescriptorEventW|PubSub_Subscribe|PubSub_Unsubscribe")
        //.allowlist_item("")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .unwrap();
}
