fn main() {
    println!("cargo:rerun-if-changed=src/app.slint");
    slint_build::compile("src/app.slint").expect("compile Slint UI");

    println!("cargo:rerun-if-changed=assets/windows/app.rc");
    println!("cargo:rerun-if-changed=src/assets/app-icon.ico");

    #[cfg(windows)]
    embed_resource::compile("assets/windows/app.rc", embed_resource::NONE)
        .manifest_optional()
        .expect("embed Windows application icon");
}
