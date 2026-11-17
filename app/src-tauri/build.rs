fn main() {
    // On Windows the app asks to run as administrator (see windows-app.manifest); elsewhere this changes nothing.
    let windows = tauri_build::WindowsAttributes::new().app_manifest(include_str!("windows-app.manifest"));
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows)).expect("failed to run the Tauri build script");
}
