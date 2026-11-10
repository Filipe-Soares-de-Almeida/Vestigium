const ICON: &str = "../ui/src-tauri/icons/icon.ico";

const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>
"#;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={ICON}");
    // Static imports resolve from System32 only, never from the helper's own folder.
    println!("cargo:rustc-link-arg-bins=/DEPENDENTLOADFLAG:0x800");
    // The debugger engine loads after main restricts the search path, so its own imports follow it.
    println!("cargo:rustc-link-arg-bins=/DELAYLOAD:dbgeng.dll");
    println!("cargo:rustc-link-arg-bins=delayimp.lib");
    // The UAC prompt and Task Manager show these fields, not the file name.
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon(ICON)
        .set_manifest(MANIFEST)
        .set("FileDescription", "Vestigium elevated helper")
        .set("ProductName", "Vestigium")
        .set("CompanyName", "Vestigium")
        .set("InternalName", "Vestigium.Helper")
        .set("OriginalFilename", "Vestigium.Helper.exe");
    // Only the binary asks for elevation; the library's test executable must run without it.
    resource
        .compile_for("vestigium-helper")
        .expect("the Windows resource compiler embeds the manifest, version info and icon");
}
