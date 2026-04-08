fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_manifest_file("windows/octa.exe.manifest");
        res.set("ProductName", "Octa");
        res.set("FileDescription", "Multi-format data viewer and editor");
        res.set("LegalCopyright", "MIT License");
        if std::path::Path::new("assets/octa.ico").exists() {
            res.set_icon("assets/octa.ico");
        }
        res.compile().expect("Failed to compile Windows resources");
    }
}
