//! Datos de versión, icono y manifiesto dentro de `otaconvert-motor.exe`.
//! SignPath exige que el nombre del producto coincida con el del proyecto
//! ("Otaconvert") y que la versión sea la misma en cada compilación.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=otaconvert.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let version = env!("CARGO_PKG_VERSION");
    let mut r = winresource::WindowsResource::new();
    // Compilación local con llvm-mingw: se indican sus herramientas.
    if let Ok(w) = std::env::var("WINDRES") {
        r.set_windres_path(&w);
    }
    if let Ok(a) = std::env::var("AR_PARA_RECURSOS") {
        r.set_ar_path(&a);
    }
    r.set("ProductName", "Otaconvert")
        .set("FileDescription", "Otaconvert - motor y Prueba de hardware")
        .set("CompanyName", "Otaconvert (proyecto libre)")
        .set("LegalCopyright", "Software libre GPLv3")
        .set("OriginalFilename", "otaconvert-motor.exe")
        .set("InternalName", "otaconvert-motor")
        .set("ProductVersion", version)
        .set("FileVersion", version)
        .set_icon("../../../assets/otaconvert.ico")
        .set_manifest(&std::fs::read_to_string("otaconvert.manifest").expect("falta otaconvert.manifest"));
    if let Err(e) = r.compile() {
        panic!("no se pudieron agregar los datos de versión al exe: {e}");
    }
}
