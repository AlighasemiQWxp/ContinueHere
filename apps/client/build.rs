fn main() {
    let configuration = slint_build::CompilerConfiguration::new().with_style("material".into());
    slint_build::compile_with_config("ui/app.slint", configuration)
        .expect("Slint interface should compile");
}
