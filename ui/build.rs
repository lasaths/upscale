fn main() {
    // Embed loku.ico into the Windows PE so Explorer / Start Menu show the mark.
    if std::env::var_os("CARGO_CFG_TARGET_OS").as_deref() == Some(std::ffi::OsStr::new("windows"))
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon/loku.ico");
        res.set("ProductName", "Loku");
        res.set("FileDescription", "Loku — image upscaler");
        if let Err(err) = res.compile() {
            // shortcut: skip embed when RC tools are missing (e.g. no MSVC),
            // upgrade when CI/local always has the Windows SDK.
            println!("cargo:warning=winresource skipped: {err}");
        }
    }
}
