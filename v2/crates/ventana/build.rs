//! Datos de versión, icono y manifiesto dentro de `otaconvert-ventana.exe`
//! (el mismo manifiesto que el motor). Ver `crates/motor/build.rs`.

fn main() {
    let manifiesto = "../motor/otaconvert.manifest";
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={manifiesto}");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let version = env!("CARGO_PKG_VERSION");
    let mut r = winresource::WindowsResource::new();
    if let Ok(w) = std::env::var("WINDRES") {
        r.set_windres_path(&w);
    }
    if let Ok(a) = std::env::var("AR_PARA_RECURSOS") {
        r.set_ar_path(&a);
    }
    r.set("ProductName", "Otaconvert")
        .set("FileDescription", "Otaconvert V2 - ventana")
        .set("CompanyName", "Otaconvert (proyecto libre)")
        .set("LegalCopyright", "Software libre GPLv3")
        .set("OriginalFilename", "otaconvert-ventana.exe")
        .set("InternalName", "otaconvert-ventana")
        .set("ProductVersion", version)
        .set("FileVersion", version)
        .set_icon("../../../assets/otaconvert.ico");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        let dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
        let ruta = std::path::Path::new(&dir).join(manifiesto);
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", ruta.display());
        println!("cargo:rustc-link-arg=/MANIFESTUAC:NO");
    } else {
        r.set_manifest(&std::fs::read_to_string(manifiesto).expect("falta otaconvert.manifest"));
    }
    if let Err(e) = r.compile() {
        panic!("no se pudieron agregar los datos de versión al exe: {e}");
    }
}
