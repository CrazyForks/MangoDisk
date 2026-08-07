fn main() {
    // Tauri embeds native icons into the compiled application, but icon-only changes do not
    // automatically invalidate the generated context in every development workflow. Watching the
    // directory ensures `tauri dev` rebuilds the executable after a Dock or taskbar icon update.
    println!("cargo:rerun-if-changed=icons");

    let windows =
        tauri_build::WindowsAttributes::new().app_manifest(include_str!("windows/app.manifest"));
    let attributes = tauri_build::Attributes::new().windows_attributes(windows);

    tauri_build::try_build(attributes).expect("failed to run the Tauri build script");
}
