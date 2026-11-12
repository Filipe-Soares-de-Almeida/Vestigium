use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::{Component, Program, texts};

/// Development packages whose code the build compiles into the window: Svelte's runtime.
const COMPILED_IN: [&str; 2] = ["svelte", "clsx"];

/// The JavaScript packages the window ships: its dependencies with theirs, and Svelte's runtime.
pub fn packages(ui: &Path) -> Result<Vec<Component>, String> {
    let manifest = read_json(&ui.join("package.json"))?;
    let mut queue: Vec<String> = dependencies(&manifest);
    queue.extend(COMPILED_IN.map(str::to_owned));
    let mut seen = BTreeSet::new();
    let mut list = Vec::new();
    while let Some(name) = queue.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        let dir = ui.join("node_modules").join(&name);
        let package = read_json(&dir.join("package.json"))
            .map_err(|e| format!("{e} (run npm ci in {})", ui.display()))?;
        let version = package["version"]
            .as_str()
            .ok_or_else(|| format!("{name} has no version"))?
            .to_owned();
        let license = license(&package).ok_or_else(|| format!("{name} declares no license"))?;
        let texts = texts::read(&texts::files_in(&dir)?)?;
        if texts.is_empty() {
            return Err(format!("{name} {version} ships no license text"));
        }
        if !COMPILED_IN.contains(&name.as_str()) {
            queue.extend(dependencies(&package));
        }
        list.push(Component {
            name,
            version,
            license,
            programs: BTreeSet::from([Program::Window]),
            main: false,
            texts,
        });
    }
    Ok(list)
}

fn dependencies(package: &Value) -> Vec<String> {
    package["dependencies"]
        .as_object()
        .map(|deps| deps.keys().cloned().collect())
        .unwrap_or_default()
}

/// The declared license: an SPDX string, or the `type` of the old object form.
fn license(package: &Value) -> Option<String> {
    let license = &package["license"];
    license
        .as_str()
        .or_else(|| license["type"].as_str())
        .map(str::to_owned)
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, json: &str, license: Option<&str>) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("package.json"), json).unwrap();
        if let Some(text) = license {
            fs::write(dir.join("LICENSE"), text).unwrap();
        }
    }

    #[test]
    fn ships_dependencies_and_the_svelte_runtime_but_not_other_dev_packages() {
        let ui = std::env::temp_dir().join(format!("vestigium-npm-{}", std::process::id()));
        let modules = ui.join("node_modules");
        write(
            &ui,
            r#"{"dependencies":{"@scope/api":"^1"},"devDependencies":{"svelte":"^5","vite":"^8"}}"#,
            None,
        );
        write(
            &modules.join("@scope/api"),
            r#"{"version":"1.2.0","license":"Apache-2.0 OR MIT","dependencies":{"tiny":"^2"}}"#,
            Some("api license"),
        );
        write(
            &modules.join("tiny"),
            r#"{"version":"2.0.1","license":{"type":"ISC"}}"#,
            Some("tiny license"),
        );
        write(
            &modules.join("svelte"),
            r#"{"version":"5.1.0","license":"MIT","dependencies":{"acorn":"^8"}}"#,
            Some("svelte license"),
        );
        write(
            &modules.join("clsx"),
            r#"{"version":"2.1.1","license":"MIT"}"#,
            Some("clsx license"),
        );
        let list = packages(&ui);
        fs::remove_dir_all(&ui).unwrap();

        let mut found: Vec<String> = list
            .unwrap()
            .iter()
            .map(|c| format!("{} {} {}", c.name, c.version, c.license))
            .collect();
        found.sort();
        assert_eq!(
            found,
            [
                "@scope/api 1.2.0 Apache-2.0 OR MIT",
                "clsx 2.1.1 MIT",
                "svelte 5.1.0 MIT",
                "tiny 2.0.1 ISC"
            ]
        );
    }

    #[test]
    fn a_package_without_a_license_text_is_refused() {
        let ui = std::env::temp_dir().join(format!("vestigium-npm-bare-{}", std::process::id()));
        write(&ui, r#"{"dependencies":{"bare":"^1"}}"#, None);
        write(
            &ui.join("node_modules/bare"),
            r#"{"version":"1.0.0","license":"MIT"}"#,
            None,
        );
        for name in COMPILED_IN {
            write(
                &ui.join("node_modules").join(name),
                r#"{"version":"1.0.0","license":"MIT"}"#,
                Some("text"),
            );
        }
        let error = packages(&ui).unwrap_err();
        fs::remove_dir_all(&ui).unwrap();
        assert!(error.contains("bare 1.0.0"), "{error}");
    }
}
