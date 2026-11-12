use serde::Serialize;

use crate::Component;

const WIDTH: usize = 80;

#[derive(Serialize)]
struct Record<'a> {
    name: &'a str,
    version: &'a str,
    license: &'a str,
    programs: Vec<&'static str>,
    main: bool,
}

/// The list as a JSON array, one component per line.
pub fn json(list: &[Component]) -> String {
    let lines: Vec<String> = list
        .iter()
        .map(|c| {
            let record = Record {
                name: &c.name,
                version: &c.version,
                license: &c.license,
                programs: c.programs.iter().map(|p| p.name()).collect(),
                main: c.main,
            };
            format!(
                "  {}",
                serde_json::to_string(&record).expect("a record serializes")
            )
        })
        .collect();
    format!("[\n{}\n]\n", lines.join(",\n"))
}

/// Every component with its license, then each distinct text once, headed by the components it
/// covers. Texts that differ only in spacing or line breaks count as one.
pub fn notices(list: &[Component], app_version: &str) -> Result<String, String> {
    let mut groups: Vec<(String, &str, Vec<String>)> = Vec::new();
    for component in list {
        if component.texts.is_empty() {
            return Err(format!(
                "{} {} has no license text",
                component.name, component.version
            ));
        }
        let label = format!("{} {}", component.name, component.version);
        for text in &component.texts {
            let key = text.split_whitespace().collect::<Vec<_>>().join(" ");
            match groups.iter_mut().find(|(k, _, _)| *k == key) {
                Some((_, _, labels)) if labels.last() != Some(&label) => labels.push(label.clone()),
                Some(_) => {}
                None => groups.push((key, text, vec![label.clone()])),
            }
        }
    }

    let mut out = format!("Third-party notices for Vestigium {app_version}\n\n");
    out.push_str(&wrap(
        "Vestigium includes the open-source components listed below. Each section after the \
         list gives one license or notice text, headed by the components it covers.",
        ' ',
    ));
    out.push_str("\nComponents\n\n");
    for c in list {
        out.push_str(&format!("  {} {} ({})\n", c.name, c.version, c.license));
    }
    for (_, text, labels) in groups {
        out.push('\n');
        out.push_str(&"=".repeat(WIDTH));
        out.push('\n');
        out.push_str(&wrap(&labels.join(", "), ','));
        out.push_str(&"-".repeat(WIDTH));
        out.push_str("\n\n");
        out.push_str(text);
        out.push('\n');
    }
    Ok(out)
}

/// `text` broken into lines of at most `WIDTH` columns after `separator`, each ending in a newline.
fn wrap(text: &str, separator: char) -> String {
    let mut out = String::new();
    let mut line = String::new();
    for word in text.split_inclusive(separator) {
        if !line.is_empty() && line.len() + word.trim_end().len() > WIDTH {
            out.push_str(line.trim_end());
            out.push('\n');
            line.clear();
        }
        line.push_str(if line.is_empty() {
            word.trim_start()
        } else {
            word
        });
    }
    if !line.trim().is_empty() {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Program, component};

    #[test]
    fn json_has_one_record_per_line_with_programs_in_order() {
        let mut first = component("adler2", "2.0.1", &[Program::Helper, Program::Window]);
        first.license = "0BSD OR MIT OR Apache-2.0".to_owned();
        let mut second = component("tauri", "2.12.5", &[Program::Window]);
        second.main = true;
        assert_eq!(
            json(&[first, second]),
            "[\n  {\"name\":\"adler2\",\"version\":\"2.0.1\",\"license\":\"0BSD OR MIT OR Apache-2.0\",\
             \"programs\":[\"window\",\"helper\"],\"main\":false},\n  \
             {\"name\":\"tauri\",\"version\":\"2.12.5\",\"license\":\"MIT\",\
             \"programs\":[\"window\"],\"main\":true}\n]\n"
        );
    }

    #[test]
    fn identical_texts_are_written_once_for_all_their_components() {
        let mut serde = component("serde", "1.0.229", &[Program::Window]);
        serde.texts = vec!["Apache text".to_owned(), "MIT for serde".to_owned()];
        let mut tauri = component("tauri", "2.12.5", &[Program::Window]);
        tauri.texts = vec!["Apache text".to_owned(), "MIT for tauri".to_owned()];
        let out = notices(&[serde, tauri], "0.1.1").unwrap();
        assert_eq!(out.matches("Apache text").count(), 1);
        assert!(out.contains("serde 1.0.229, tauri 2.12.5\n----"), "{out}");
        assert!(out.contains("serde 1.0.229\n----"), "{out}");
        assert!(out.starts_with("Third-party notices for Vestigium 0.1.1\n"));
        assert!(out.contains("  serde 1.0.229 (MIT)\n"));
    }

    #[test]
    fn a_component_without_text_stops_the_notices() {
        let mut bare = component("bare", "1.0.0", &[Program::Window]);
        bare.texts.clear();
        assert!(
            notices(&[bare], "0.1.1")
                .unwrap_err()
                .contains("bare 1.0.0")
        );
    }

    #[test]
    fn long_heads_wrap_after_a_comma() {
        let labels: Vec<String> = (0..30).map(|n| format!("crate-{n} 1.0.{n}")).collect();
        let wrapped = wrap(&labels.join(", "), ',');
        assert!(wrapped.lines().count() > 1);
        assert!(wrapped.lines().all(|line| line.len() <= WIDTH), "{wrapped}");
        assert!(
            wrapped
                .lines()
                .rev()
                .skip(1)
                .all(|line| line.ends_with(','))
        );
        assert_eq!(
            wrapped.replace(",\n", ", "),
            format!("{}\n", labels.join(", "))
        );
    }
}
