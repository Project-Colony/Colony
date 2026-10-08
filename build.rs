// Windows version resource: SignPath signs only an .exe whose ProductName is
// the project name and whose ProductVersion is set, and the release workflow
// checks both on the built file. Decided on the TARGET, not with cfg!(windows):
// build scripts run on the host.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let version = env!("CARGO_PKG_VERSION");
        winresource::WindowsResource::new()
            .set("ProductName", "Colony")
            .set("FileDescription", "Colony")
            .set("ProductVersion", version)
            .set("FileVersion", version)
            .compile()
            .expect("could not embed the Windows version resource");
    }
}
