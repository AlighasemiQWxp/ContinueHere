fn main() {
    let configuration = slint_build::CompilerConfiguration::new()
        .with_style("material".into())
        .with_library_paths(std::collections::HashMap::from([(
            "material".into(),
            std::path::PathBuf::from(
                std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"),
            )
            .join("vendor/material/material.slint"),
        )]));
    slint_build::compile_with_config("ui/app.slint", configuration)
        .expect("Slint interface should compile");
    println!("cargo:rerun-if-changed=assets/ContinueHere.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/ContinueHere.ico")
            .set("ProductName", "ContinueHere")
            .set("FileDescription", "ContinueHere")
            .compile()
            .expect("Windows application resources should compile");
    }
}
