//! Gives the Windows executable its icon. GPUI's window uses the same resource.

fn main() {
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        println!("cargo:rerun-if-changed=assets/app_icon.ico");
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon_with_id("assets/app_icon.ico", "1");
        resource.set("LegalCopyright", "© 2026 Moritz Maibaum");
        if let Err(error) = resource.compile() {
            println!("cargo:warning=no icon in the executable: {error}");
        }
    }
}
