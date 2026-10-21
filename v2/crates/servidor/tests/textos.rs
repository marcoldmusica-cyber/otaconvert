//! Los textos de la interfaz (ui/textos.json): los dos idiomas completos y
//! sin mezclar, consejos con ejemplos de escena y cortos (regla de interfaz de
//! CLAUDE.md), y todo lo que usa la página existe.

use serde_json::{Map, Value};

const PAGINA: &str = include_str!("../ui/index.html");

fn textos() -> Value {
    serde_json::from_str(otaconvert_servidor::textos()).expect("ui/textos.json no es JSON válido")
}

fn obj<'a>(v: &'a Value, k: &str) -> &'a Map<String, Value> {
    v[k].as_object().unwrap_or_else(|| panic!("falta «{k}»"))
}

fn claves(m: &Map<String, Value>) -> Vec<&String> {
    let mut k: Vec<_> = m.keys().collect();
    k.sort();
    k
}

#[test]
fn los_dos_idiomas_tienen_lo_mismo() {
    let t = textos();
    for parte in ["ui", "consejos", "ayuda"] {
        let (es, en) = (obj(&t["es"], parte), obj(&t["en"], parte));
        assert_eq!(claves(es), claves(en), "«{parte}»: claves distintas entre es y en");
    }
    assert_eq!(t["es"]["pasos"].as_array().unwrap().len(), t["en"]["pasos"].as_array().unwrap().len());
    // Todo con texto.
    for idioma in ["es", "en"] {
        for (k, v) in obj(&t[idioma], "ui") {
            assert!(!v.as_str().unwrap().trim().is_empty(), "{idioma}.ui.{k} vacío");
        }
    }
}

#[test]
fn consejos_completos_con_ejemplos_y_cortos() {
    let t = textos();
    let campos = ["titulo", "que_hace", "activar", "ejemplo_activar", "desactivar", "ejemplo_desactivar"];
    for (k, es) in obj(&t["es"], "consejos") {
        let en = &t["en"]["consejos"][k];
        for c in campos {
            let (a, b) = (es[c].as_str().unwrap_or(""), en[c].as_str().unwrap_or(""));
            assert!(!a.trim().is_empty() && !b.trim().is_empty(), "consejo {k}.{c} vacío");
            assert!(a.chars().count() <= 200 && b.chars().count() <= 200, "consejo {k}.{c} demasiado largo");
            assert_ne!(a, b, "consejo {k}.{c} sin traducir");
        }
        for c in ["ejemplo_activar", "ejemplo_desactivar"] {
            assert!(es[c].as_str().unwrap().starts_with("Ej.: "), "{k}.{c} (es) sin «Ej.: »");
            assert!(en[c].as_str().unwrap().starts_with("E.g.: "), "{k}.{c} (en) sin «E.g.: »");
        }
    }
}

#[test]
fn ayuda_y_pasos_traducidos() {
    let t = textos();
    for (k, es) in obj(&t["es"], "ayuda") {
        let en = &t["en"]["ayuda"][k];
        for i in 0..2 {
            let (a, b) = (es[i].as_str().unwrap(), en[i].as_str().unwrap());
            assert!(!a.is_empty() && !b.is_empty(), "ayuda {k}[{i}] vacía");
            assert!(a.chars().count() <= 400 && b.chars().count() <= 400, "ayuda {k} demasiado larga para una burbuja");
        }
        // El título puede coincidir (GO es GO en los dos); la explicación no.
        assert_ne!(es[1], en[1], "ayuda {k} sin traducir");
    }
    for (a, b) in t["es"]["pasos"].as_array().unwrap().iter().zip(t["en"]["pasos"].as_array().unwrap()) {
        assert_ne!(a, b, "paso sin traducir");
    }
}

/// Saca los nombres que siguen a `marca` hasta la comilla: T("x"), data-t="x"…
fn usados(marca: &str) -> Vec<String> {
    PAGINA.split(marca).skip(1).filter_map(|r| r.split('"').next().map(str::to_string)).collect()
}

#[test]
fn todo_lo_que_usa_la_pagina_existe() {
    let t = textos();
    let ui = obj(&t["es"], "ui");
    let mut n = 0;
    for k in usados("T(\"").into_iter().chain(usados("data-t=\"")).chain(usados("data-t-title=\"")) {
        assert!(ui.contains_key(&k), "la página usa el texto «{k}», que no está en textos.json");
        n += 1;
    }
    for k in usados("data-help=\"").into_iter().chain(usados("ayudaDe(\"")) {
        assert!(t["es"]["ayuda"].get(&k).is_some(), "la página usa la ayuda «{k}», que no existe");
    }
    for k in usados("data-consejo=\"").into_iter().chain(usados("botonConsejo(\"")) {
        assert!(t["es"]["consejos"].get(&k).is_some(), "la página usa el consejo «{k}», que no existe");
    }
    assert!(n > 50, "la página casi no usa textos traducidos ({n})");
    assert!(PAGINA.contains("\"__TEXTOS__\""), "falta el lugar donde van los textos");
}

#[test]
fn ningun_texto_puede_cerrar_el_script() {
    // El servidor escapa «</»; aquí se comprueba que el JSON resultante sigue siendo válido.
    let escapado = otaconvert_servidor::textos().replace("</", "<\\/");
    assert!(!escapado.contains("</"));
    serde_json::from_str::<Value>(&escapado).unwrap();
}
