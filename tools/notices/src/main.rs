//! Lists the third-party components the window, the monitor and the helper ship.
//!
//! `vestigium-notices` writes `apps/ui/src-tauri/third-party.json`; `--check` fails when that file
//! no longer matches the dependency graph; `--notices <file>` also writes the license notices.

mod cargo;
mod extras;
mod npm;
mod render;
mod texts;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The list the window reads, relative to the workspace root.
const LIST: &str = "apps/ui/src-tauri/third-party.json";

/// Components the About screen shows first: the ones Vestigium is built on.
const MAIN: [&str; 20] = [
    "tauri",
    "wry",
    "tao",
    "webview2-com",
    "tauri-plugin-updater",
    "tauri-plugin-single-instance",
    "@tauri-apps/api",
    "svelte",
    "uplot",
    "windows",
    "rusqlite",
    extras::SQLITE,
    "tray-icon",
    "serde",
    "serde_json",
    "postcard",
    "miniz_oxide",
    "thiserror",
    extras::PRESENTMON,
    extras::RUST_STD,
];

/// A shipped program, in the order the list names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Program {
    Window,
    Monitor,
    Helper,
}

impl Program {
    pub const ALL: [Program; 3] = [Program::Window, Program::Monitor, Program::Helper];

    pub fn name(self) -> &'static str {
        match self {
            Program::Window => "window",
            Program::Monitor => "monitor",
            Program::Helper => "helper",
        }
    }
}

/// One third-party package or product, and the license texts that go with it.
#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    pub name: String,
    pub version: String,
    pub license: String,
    pub programs: BTreeSet<Program>,
    pub main: bool,
    pub texts: Vec<String>,
}

fn main() -> ExitCode {
    let mut check = false;
    let mut notices = None;
    let mut args = std::env::args().skip(1);
    let mut result = None;
    while let Some(arg) = args.next() {
        match (arg.as_str(), notices.is_none()) {
            ("--check", _) => check = true,
            ("--notices", true) => match args.next() {
                Some(path) => notices = Some(PathBuf::from(path)),
                None => result = Some(Err(USAGE.to_owned())),
            },
            _ => result = Some(Err(USAGE.to_owned())),
        }
    }
    match result.unwrap_or_else(|| run(check, notices.as_deref())) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

const USAGE: &str = "usage: vestigium-notices [--check] [--notices <file>]";

fn run(check: bool, notices: Option<&Path>) -> Result<bool, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("the tool is not inside the workspace")?;
    let graph = cargo::crates(root)?;
    let packages = npm::packages(&root.join("apps/ui"))?;
    let others = extras::all(root, &graph.crates)?;
    let summary = summary(&graph.crates, packages.len(), others.len());
    let mut list: Vec<Component> = graph.crates.into_iter().map(|c| c.component).collect();
    list.extend(packages);
    list.extend(others);
    mark_main(&mut list)?;
    sort(&mut list);

    // Built on every run, so a check also proves that every component has its license text.
    let text = render::notices(&list, &graph.app_version)?;
    let json = render::json(&list);
    let path = root.join(LIST);
    if check {
        let current = fs::read_to_string(&path).unwrap_or_default();
        if current.replace('\r', "") != json {
            eprintln!(
                "{LIST} does not match the dependency graph. Run `cargo run -p vestigium-notices` \
                 and commit the result."
            );
            return Ok(false);
        }
    } else {
        fs::write(&path, &json).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    if let Some(out) = notices {
        fs::write(out, text.replace('\n', "\r\n"))
            .map_err(|e| format!("{}: {e}", out.display()))?;
    }
    let main = list.iter().filter(|c| c.main).count();
    println!("{summary}, {main} of them main.");
    Ok(true)
}

/// Marks the components named in `MAIN`, and fails when one of them is no longer shipped.
fn mark_main(list: &mut [Component]) -> Result<(), String> {
    for name in MAIN {
        let mut found = false;
        for component in list.iter_mut().filter(|c| c.name == name) {
            component.main = true;
            found = true;
        }
        if !found {
            return Err(format!(
                "{name} is in the main list but no program ships it"
            ));
        }
    }
    Ok(())
}

/// Sorts by name, ignoring case and a leading `@`, then by version.
fn sort(list: &mut [Component]) {
    list.sort_by(|a, b| {
        let key = |c: &Component| c.name.trim_start_matches('@').to_ascii_lowercase();
        key(a)
            .cmp(&key(b))
            .then_with(|| version_key(&a.version).cmp(&version_key(&b.version)))
            .then_with(|| a.version.cmp(&b.version))
    });
}

fn version_key(version: &str) -> Vec<u64> {
    version
        .split(['.', '-', '+'])
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

fn summary(crates: &[cargo::Crate], packages: usize, others: usize) -> String {
    let linked = |program: Program| {
        crates
            .iter()
            .filter(|c| c.component.programs.contains(&program))
            .count()
    };
    format!(
        "{} Rust crates (window {}, monitor {}, helper {}), {packages} npm packages, {others} others",
        crates.len(),
        linked(Program::Window),
        linked(Program::Monitor),
        linked(Program::Helper),
    )
}

#[cfg(test)]
pub fn component(name: &str, version: &str, programs: &[Program]) -> Component {
    Component {
        name: name.to_owned(),
        version: version.to_owned(),
        license: "MIT".to_owned(),
        programs: programs.iter().copied().collect(),
        main: false,
        texts: vec![format!("{name} license")],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_ignores_case_and_the_scope_sign_and_orders_versions_by_number() {
        let mut list = vec![
            component("uplot", "1.6.32", &[Program::Window]),
            component("getrandom", "0.10.0", &[Program::Window]),
            component("@tauri-apps/api", "2.12.4", &[Program::Window]),
            component("getrandom", "0.9.1", &[Program::Window]),
            component("SQLite", "3.54.1", &[Program::Window]),
        ];
        sort(&mut list);
        let names: Vec<String> = list
            .iter()
            .map(|c| format!("{} {}", c.name, c.version))
            .collect();
        assert_eq!(
            names,
            [
                "getrandom 0.9.1",
                "getrandom 0.10.0",
                "SQLite 3.54.1",
                "@tauri-apps/api 2.12.4",
                "uplot 1.6.32"
            ]
        );
    }

    #[test]
    fn mark_main_flags_every_version_of_a_main_name() {
        let mut list: Vec<Component> = MAIN
            .iter()
            .map(|name| component(name, "1.0.0", &[Program::Window]))
            .collect();
        list.push(component("windows", "0.61.0", &[Program::Helper]));
        list.push(component("cfg-if", "1.0.0", &[Program::Window]));
        mark_main(&mut list).unwrap();
        assert!(list.iter().filter(|c| c.name == "windows").all(|c| c.main));
        assert!(!list.iter().find(|c| c.name == "cfg-if").unwrap().main);
    }

    #[test]
    fn mark_main_fails_when_a_main_component_is_gone() {
        let mut list = vec![component("tauri", "2.0.0", &[Program::Window])];
        let error = mark_main(&mut list).unwrap_err();
        assert!(error.contains("wry"), "{error}");
    }
}
