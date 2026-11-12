use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::{Component, Program, texts};

const TARGET: &str = "x86_64-pc-windows-msvc";

/// The package each shipped program is built from.
const ROOTS: [(Program, &str); 3] = [
    (Program::Window, "vestigium-ui"),
    (Program::Monitor, "vestigium-agent"),
    (Program::Helper, "vestigium-helper"),
];

/// Texts kept in the repository for packages that ship without a license file.
pub const KEPT_TEXTS: &str = "tools/notices/texts";

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    version: String,
    source: Option<String>,
    license: Option<String>,
    license_file: Option<PathBuf>,
    manifest_path: PathBuf,
}

/// A third-party crate linked into at least one program, and the folder of its source.
pub struct Crate {
    pub component: Component,
    pub dir: PathBuf,
}

pub struct Graph {
    pub crates: Vec<Crate>,
    pub app_version: String,
}

/// The registry and git crates the three programs link: normal dependencies, no proc-macros.
pub fn crates(root: &Path) -> Result<Graph, String> {
    let metadata = cargo(
        root,
        &[
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--filter-platform",
            TARGET,
        ],
    )?;
    let metadata: Metadata =
        serde_json::from_str(&metadata).map_err(|e| format!("cargo metadata: {e}"))?;

    let mut linked: BTreeMap<(String, String), BTreeSet<Program>> = BTreeMap::new();
    for (program, package) in ROOTS {
        let tree = cargo(
            root,
            &[
                "tree",
                "--locked",
                "--package",
                package,
                "--edges",
                "normal,no-proc-macro",
                "--target",
                TARGET,
                "--prefix",
                "none",
                "--format",
                "{p}",
            ],
        )?;
        for id in tree_packages(&tree) {
            linked.entry(id).or_default().insert(program);
        }
    }

    let app_version = find(&metadata, ROOTS[0].1, None)?.version.clone();
    let mut crates = Vec::new();
    for ((name, version), programs) in linked {
        let package = find(&metadata, &name, Some(&version))?;
        if package.source.is_none() {
            continue;
        }
        let dir = package
            .manifest_path
            .parent()
            .ok_or_else(|| format!("{name} {version}: no package folder"))?
            .to_path_buf();
        let license = package
            .license
            .clone()
            .ok_or_else(|| format!("{name} {version} declares no SPDX license"))?;
        let texts = crate_texts(root, &name, &dir, package.license_file.as_deref())?;
        if texts.is_empty() {
            return Err(format!(
                "{name} {version} ships no license text: add it under {KEPT_TEXTS}/{name}/"
            ));
        }
        let component = Component {
            name,
            version,
            license,
            programs,
            main: false,
            texts,
        };
        crates.push(Crate { component, dir });
    }
    Ok(Graph {
        crates,
        app_version,
    })
}

/// The crate's own license files, its declared license file, then the texts kept for it here.
fn crate_texts(
    root: &Path,
    name: &str,
    dir: &Path,
    license_file: Option<&Path>,
) -> Result<Vec<String>, String> {
    let mut files = texts::files_in(dir)?;
    if let Some(file) = license_file.map(|file| dir.join(file))
        && file.is_file()
        && !files.contains(&file)
    {
        files.push(file);
    }
    files.extend(texts::all_files_in(&root.join(KEPT_TEXTS).join(name))?);
    texts::read(&files)
}

fn find<'a>(
    metadata: &'a Metadata,
    name: &str,
    version: Option<&str>,
) -> Result<&'a Package, String> {
    metadata
        .packages
        .iter()
        .find(|p| p.name == name && version.is_none_or(|v| p.version == v))
        .ok_or_else(|| {
            format!(
                "cargo metadata has no package {name} {}",
                version.unwrap_or("")
            )
        })
}

/// The name and version of each line `cargo tree --prefix none --format {p}` prints.
fn tree_packages(tree: &str) -> BTreeSet<(String, String)> {
    tree.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            let version = words.next()?.strip_prefix('v')?;
            Some((name.to_owned(), version.to_owned()))
        })
        .collect()
}

fn cargo(root: &Path, args: &[&str]) -> Result<String, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("cargo {}: {e}", args[0]))?;
    if !output.status.success() {
        return Err(format!(
            "cargo {} failed:\n{}",
            args[0],
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("cargo {}: {e}", args[0]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_lines_give_name_and_version_once() {
        let tree = "vestigium-ui v0.1.1 (C:\\src\\apps\\ui\\src-tauri)\n\
                    serde v1.0.229\n\
                    toml v1.1.8+spec-1.1.0\n\
                    serde v1.0.229 (*)\n\
                    \n\
                    getrandom v0.3.4\n";
        let ids: Vec<String> = tree_packages(tree)
            .into_iter()
            .map(|(name, version)| format!("{name} {version}"))
            .collect();
        assert_eq!(
            ids,
            [
                "getrandom 0.3.4",
                "serde 1.0.229",
                "toml 1.1.8+spec-1.1.0",
                "vestigium-ui 0.1.1"
            ]
        );
    }

    #[test]
    fn kept_texts_complete_a_crate_that_ships_none() {
        let root = std::env::temp_dir().join(format!("vestigium-kept-{}", std::process::id()));
        let dir = root.join("registry/plain-1.0.0");
        let kept = root.join(KEPT_TEXTS).join("plain");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(&kept).unwrap();
        std::fs::write(dir.join("README.md"), "Read me").unwrap();
        std::fs::write(kept.join("LICENSE"), "Kept license").unwrap();
        let texts = crate_texts(&root, "plain", &dir, None);
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(texts.unwrap(), vec!["Kept license".to_owned()]);
    }

    #[test]
    fn a_declared_license_file_outside_the_usual_names_is_read() {
        let root = std::env::temp_dir().join(format!("vestigium-declared-{}", std::process::id()));
        let dir = root.join("registry/odd-1.0.0");
        std::fs::create_dir_all(dir.join("legal")).unwrap();
        std::fs::write(dir.join("LICENSE-MIT"), "MIT text").unwrap();
        std::fs::write(dir.join("legal/terms.txt"), "Odd terms").unwrap();
        let texts = crate_texts(&root, "odd", &dir, Some(Path::new("legal/terms.txt")));
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            texts.unwrap(),
            vec!["MIT text".to_owned(), "Odd terms".to_owned()]
        );
    }
}
