use std::collections::{BTreeSet, HashMap};

use argos_i18n::{Catalog, Lang};

fn keys(lang: Lang) -> BTreeSet<String> {
    Catalog::load(lang).keys().map(str::to_string).collect()
}

#[test]
fn catalogs_have_the_same_keys() {
    let en = keys(Lang::En);
    let pt = keys(Lang::PtBr);
    assert!(!en.is_empty());
    assert_eq!(
        en.difference(&pt).collect::<Vec<_>>(),
        Vec::<&String>::new(),
        "missing in pt-BR"
    );
    assert_eq!(
        pt.difference(&en).collect::<Vec<_>>(),
        Vec::<&String>::new(),
        "missing in en"
    );
}

#[test]
fn no_value_is_empty() {
    for lang in [Lang::En, Lang::PtBr] {
        let catalog = Catalog::load(lang);
        for key in catalog.keys() {
            assert!(
                !catalog.get(key).trim().is_empty(),
                "{lang:?}: {key} is empty"
            );
        }
    }
}

#[test]
fn each_language_has_its_own_text() {
    let en = Catalog::load(Lang::En);
    let pt = Catalog::load(Lang::PtBr);
    assert_ne!(en.get("tray.exit"), pt.get("tray.exit"));
}

#[test]
fn missing_key_falls_back_to_en() {
    let en = HashMap::from([("a".to_string(), "alpha".to_string())]);
    let other = HashMap::from([("b".to_string(), "beta".to_string())]);
    let catalog = Catalog::from_maps(other, en);

    assert_eq!(catalog.get("a"), "alpha");
    assert_eq!(catalog.get("b"), "beta");
}

#[test]
fn unknown_key_returns_the_key() {
    assert_eq!(Catalog::load(Lang::En).get("no.such.key"), "no.such.key");
}

#[test]
fn from_langid_maps_portuguese() {
    assert_eq!(Lang::from_langid(0x0416), Lang::PtBr);
    assert_eq!(Lang::from_langid(0x0816), Lang::PtBr);
    assert_eq!(Lang::from_langid(0x0409), Lang::En);
    assert_eq!(Lang::from_langid(0x0C0A), Lang::En);
}

#[test]
fn format_replaces_placeholders() {
    let en = HashMap::from([("k".to_string(), "x {error} y {error}".to_string())]);
    let catalog = Catalog::from_maps(HashMap::new(), en);

    assert_eq!(catalog.format("k", &[("error", "boom")]), "x boom y boom");
}
