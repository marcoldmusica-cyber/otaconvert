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
        .set("FileDescription", "Otaconvert V2 - motor y Prueba de hardware")
        .set("CompanyName", "Otaconvert (proyecto libre)")
        .set("LegalCopyright", "Software libre GPLv3")
        .set("OriginalFilename", "otaconvert-motor.exe")
        .set("InternalName", "otaconvert-motor")
        .set("ProductVersion", version)
        .set("FileVersion", version)
        .set_icon("../../../assets/otaconvert.ico");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        // Con MSVC el manifiesto lo inserta el enlazador (vía documentada por
        // Microsoft). Meterlo como recurso con rc.exe dejó un manifiesto que
        // Windows rechazaba ("side-by-side configuration is incorrect").
        // /MANIFESTUAC:NO porque nuestro manifiesto ya dice asInvoker.
        let dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
        let ruta = std::path::Path::new(&dir).join("otaconvert.manifest");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", ruta.display());
        println!("cargo:rustc-link-arg=/MANIFESTUAC:NO");
    } else {
        r.set_manifest(&std::fs::read_to_string("otaconvert.manifest").expect("falta otaconvert.manifest"));
    }
    if let Err(e) = r.compile() {
        panic!("no se pudieron agregar los datos de versión al exe: {e}");
    }
}
