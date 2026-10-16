fn main() {
    println!("cargo:rerun-if-changed=assets/app.ico");
    println!("cargo:rerun-if-changed=ui/app.slint");
    slint_build::compile_with_config(
        "ui/app.slint",
        slint_build::CompilerConfiguration::new().with_style("fluent".into()),
    )
    .expect("failed to compile the Slint settings window");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").expect("Cargo target OS is required");
    if target_os == "windows" {
        winresource::WindowsResource::new()
            .set_icon("assets/app.ico")
            .set("FileDescription", "Audio Shortcut Switch")
            .set("ProductName", "Audio Shortcut Switch")
            .compile()
            .expect("failed to embed the PON application icon");
    }
}
