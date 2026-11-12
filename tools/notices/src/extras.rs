use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cargo::Crate;
use crate::{Component, Program, texts};

pub const SQLITE: &str = "SQLite";
pub const PRESENTMON: &str = "PresentMon";
pub const RUST_STD: &str = "Rust standard library";

/// PresentMon's own files, which the installer already puts beside the programs.
const PRESENTMON_NOTICE: &str = "\
PresentMon's license and its own third-party notices are installed beside the
programs, in licenses\\PresentMon\\LICENSE.txt and
licenses\\PresentMon\\THIRD_PARTY.txt.";

const RUST_STD_NOTICE: &str = "\
The Rust standard library is licensed under the Apache License, Version 2.0, or
the MIT license, at your option. Copyrights in it are retained by their
contributors, the Rust Project Developers (see https://thanks.rust-lang.org).";

/// The placeholder line of the MIT template the toolchain ships, and the holder it stands for.
const MIT_TEMPLATE: &str = "Copyright (c) <year> <copyright holders>";
const RUST_HOLDER: &str = "Copyright (c) The Rust Project Developers";

/// The components no package manager lists: SQLite, PresentMon and the Rust standard library.
pub fn all(root: &Path, crates: &[Crate]) -> Result<Vec<Component>, String> {
    let mut list = Vec::new();
    if let Some(sys) = crates.iter().find(|c| c.component.name == "libsqlite3-sys") {
        list.push(sqlite(sys)?);
    }
    list.push(presentmon(root)?);
    list.push(rust_std(root)?);
    Ok(list)
}

/// The SQLite amalgamation `libsqlite3-sys` compiles in, under the blessing in its header.
fn sqlite(sys: &Crate) -> Result<Component, String> {
    let header = read(&sys.dir.join("sqlite3").join("sqlite3.h"))?;
    let version = define(&header, "SQLITE_VERSION").ok_or("sqlite3.h has no SQLITE_VERSION")?;
    let text = blessing(&header).ok_or("sqlite3.h has no blessing")?;
    Ok(Component {
        name: SQLITE.to_owned(),
        version,
        license: "blessing".to_owned(),
        programs: sys.component.programs.clone(),
        main: false,
        texts: vec![text],
    })
}

/// The PresentMon console the helper runs, in the version the fetch script pins.
fn presentmon(root: &Path) -> Result<Component, String> {
    let script = read(&root.join("tools").join("fetch-presentmon.ps1"))?;
    let version = assigned(&script, "$version").ok_or("fetch-presentmon.ps1 pins no $version")?;
    Ok(Component {
        name: PRESENTMON.to_owned(),
        version,
        license: "MIT".to_owned(),
        programs: BTreeSet::from([Program::Helper]),
        main: false,
        texts: vec![PRESENTMON_NOTICE.to_owned()],
    })
}

/// The standard library every program links, in the version the toolchain file pins.
fn rust_std(root: &Path) -> Result<Component, String> {
    let toolchain = read(&root.join("rust-toolchain.toml"))?;
    let version = assigned(&toolchain, "channel").ok_or("rust-toolchain.toml pins no channel")?;
    let licenses = sysroot(root)?
        .join("share")
        .join("doc")
        .join("rust")
        .join("licenses");
    let mut texts = vec![RUST_STD_NOTICE.to_owned()];
    texts.extend(
        texts::read(&[licenses.join("Apache-2.0.txt"), licenses.join("MIT.txt")])?
            .into_iter()
            .map(|text| text.replace(MIT_TEMPLATE, RUST_HOLDER)),
    );
    Ok(Component {
        name: RUST_STD.to_owned(),
        version,
        license: "MIT OR Apache-2.0".to_owned(),
        programs: Program::ALL.into_iter().collect(),
        main: false,
        texts,
    })
}

fn sysroot(root: &Path) -> Result<PathBuf, String> {
    let output = Command::new("rustc")
        .args(["--print", "sysroot"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("rustc --print sysroot: {e}"))?;
    if !output.status.success() {
        return Err("rustc --print sysroot failed".to_owned());
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim(),
    ))
}

/// The quoted value of `#define NAME "value"`.
fn define(header: &str, name: &str) -> Option<String> {
    header.lines().find_map(|line| {
        let rest = line.strip_prefix("#define")?.trim_start();
        let value = rest.strip_prefix(name)?;
        value.starts_with([' ', '\t']).then(|| unquote(value))?
    })
}

/// The quoted value of `name = 'value'` or `name = "value"` at the start of a line.
fn assigned(text: &str, name: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let value = line.trim_start().strip_prefix(name)?.trim_start();
        unquote(value.strip_prefix('=')?)
    })
}

fn unquote(value: &str) -> Option<String> {
    let value = value.trim();
    let quote = value.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let inner = &value[1..];
    Some(inner[..inner.find(quote)?].to_owned())
}

/// The dedication at the top of an SQLite source file, without the comment markers.
fn blessing(header: &str) -> Option<String> {
    let lines: Vec<&str> = header
        .lines()
        .skip_while(|line| !line.contains("The author disclaims copyright"))
        .take_while(|line| !line.starts_with("*****"))
        .map(|line| {
            let line = line.strip_prefix("**").unwrap_or(line);
            line.strip_prefix(' ').unwrap_or(line)
        })
        .collect();
    let text = texts::normalize(lines.join("\n").as_bytes());
    (!text.is_empty()).then_some(text)
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "/*\n\
        ** 2001-09-15\n\
        **\n\
        ** The author disclaims copyright to this source code.  In place of\n\
        ** a legal notice, here is a blessing:\n\
        **\n\
        **    May you do good and not evil.\n\
        **\n\
        *************************************************************************\n\
        ** This header file defines the interface.\n\
        */\n\
        #define SQLITE_VERSION_NUMBER 3054001\n\
        #define SQLITE_VERSION        \"3.54.1\"\n";

    #[test]
    fn sqlite_version_comes_from_its_define() {
        assert_eq!(define(HEADER, "SQLITE_VERSION").as_deref(), Some("3.54.1"));
        assert_eq!(define(HEADER, "SQLITE_SOURCE_ID"), None);
    }

    #[test]
    fn sqlite_blessing_is_read_without_comment_markers() {
        assert_eq!(
            blessing(HEADER).unwrap(),
            "The author disclaims copyright to this source code.  In place of\n\
             a legal notice, here is a blessing:\n\
             \n   May you do good and not evil."
        );
    }

    #[test]
    fn pinned_versions_are_read_from_scripts_and_toml() {
        let script = "$ErrorActionPreference = 'Stop'\n\n$version = '2.6.0'\n$root = 'x'\n";
        assert_eq!(assigned(script, "$version").as_deref(), Some("2.6.0"));
        let toolchain = "[toolchain]\nchannel = \"1.99.1\"\ncomponents = [\"rustfmt\"]\n";
        assert_eq!(assigned(toolchain, "channel").as_deref(), Some("1.99.1"));
        assert_eq!(assigned(toolchain, "targets"), None);
    }
}
