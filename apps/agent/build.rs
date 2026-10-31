fn main() {
    // Task Manager and the startup apps list show these fields, not the file name.
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("../ui/src-tauri/icons/icon.ico")
        .set("FileDescription", "Vestigium")
        .set("ProductName", "Vestigium")
        .set("InternalName", "Vestigium")
        .set("OriginalFilename", "Vestigium.exe");
    resource
        .compile()
        .expect("the Windows resource compiler embeds the version info and icon");
}
