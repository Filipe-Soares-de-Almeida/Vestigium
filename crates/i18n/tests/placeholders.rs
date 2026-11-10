use std::collections::{BTreeSet, HashMap};

use vestigium_i18n::{Catalog, Lang};

/// Placeholders that usually hold a drive letter such as "C:".
const DRIVE_NAMES: [&str; 3] = ["disk", "volume", "subject"];

/// The `{name}` variables of a text: a name is one or more ASCII letters, digits or underscores.
fn placeholders(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('}') else { break };
        let name = &rest[..close];
        if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            found.insert(name.to_string());
        }
    }
    found
}

/// One message per key whose placeholders in `other` differ from English.
fn mismatches(en: &Catalog, other: &Catalog) -> Vec<String> {
    let mut keys: Vec<&str> = en.keys().collect();
    keys.sort_unstable();
    keys.into_iter()
        .filter_map(|key| {
            let (english, translated) = (placeholders(en.get(key)), placeholders(other.get(key)));
            (english != translated).then(|| {
                format!("{key}: English has {english:?}, the other language {translated:?}")
            })
        })
        .collect()
}

/// One message per text where a drive placeholder is followed by a colon, which renders "C::".
fn drive_colons(catalog: &Catalog) -> Vec<String> {
    let mut keys: Vec<&str> = catalog.keys().collect();
    keys.sort_unstable();
    keys.into_iter()
        .flat_map(|key| {
            DRIVE_NAMES
                .iter()
                .filter(move |name| catalog.get(key).contains(&format!("{{{name}}}:")))
                .map(move |name| format!("{key}: {{{name}}} is followed by a colon"))
        })
        .collect()
}

#[test]
fn placeholders_are_the_same_in_every_language() {
    let problems = mismatches(&Catalog::load(Lang::En), &Catalog::load(Lang::PtBr));

    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn a_drive_placeholder_is_never_followed_by_a_colon() {
    for lang in [Lang::En, Lang::PtBr] {
        let problems = drive_colons(&Catalog::load(lang));

        assert!(problems.is_empty(), "{lang:?}:\n{}", problems.join("\n"));
    }
}

fn catalog(entries: &[(&str, &str)]) -> Catalog {
    let text: HashMap<String, String> = entries
        .iter()
        .map(|(key, text)| (key.to_string(), text.to_string()))
        .collect();
    Catalog::from_maps(text, HashMap::new())
}

#[test]
fn a_missing_placeholder_is_reported() {
    let en = catalog(&[("k", "{a} then {b}")]);
    let other = catalog(&[("k", "{a} only")]);

    let problems = mismatches(&en, &other);

    assert_eq!(problems.len(), 1);
    assert!(problems[0].starts_with("k:"), "{problems:?}");
    assert!(problems[0].contains("\"b\""), "{problems:?}");
}

#[test]
fn a_renamed_placeholder_is_reported() {
    let en = catalog(&[("k", "{count} items")]);
    let other = catalog(&[("k", "{total} items")]);

    assert_eq!(mismatches(&en, &other).len(), 1);
}

#[test]
fn equal_placeholders_in_another_order_are_not_reported() {
    let en = catalog(&[("k", "{a} before {b}")]);
    let other = catalog(&[("k", "{b} after {a} and {a}")]);

    assert!(mismatches(&en, &other).is_empty());
}

#[test]
fn a_brace_without_a_name_is_not_a_placeholder() {
    assert!(placeholders("{}").is_empty());
    assert!(placeholders("{ x }").is_empty());
    assert!(placeholders("{a-b}").is_empty());
    assert!(placeholders("{ open").is_empty());
    assert_eq!(
        placeholders("{{name}}"),
        BTreeSet::from(["name".to_string()])
    );
    assert_eq!(
        placeholders("{a_1} and {B2}"),
        BTreeSet::from(["a_1".to_string(), "B2".to_string()])
    );
}

#[test]
fn a_drive_placeholder_before_a_colon_is_reported() {
    let bad = catalog(&[
        ("a", "Free space on {disk}: low"),
        ("b", "Volume {volume}: full"),
        ("c", "Disk {subject}: failing"),
        ("d", "{app}: crashed"),
        ("e", "On {disk}, {volume} · {subject}"),
    ]);

    let problems = drive_colons(&bad);

    assert_eq!(problems.len(), 3, "{problems:?}");
    assert!(
        problems
            .iter()
            .all(|p| !p.starts_with("d:") && !p.starts_with("e:"))
    );
}
