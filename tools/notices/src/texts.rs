use std::fs;
use std::path::{Path, PathBuf};

/// Names a license, copying, copyright or notice file starts with.
const STEMS: [&str; 6] = [
    "license",
    "licence",
    "unlicense",
    "copying",
    "copyright",
    "notice",
];
/// Extensions of files that only describe a license, such as SPDX documents, or are source code.
const NOT_TEXT: [&str; 7] = [".spdx", ".html", ".htm", ".rs", ".toml", ".json", ".js"];

/// True when a file named `name` holds a license or notice text.
pub fn is_license_file(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    STEMS.iter().any(|stem| name.starts_with(stem))
        && !NOT_TEXT.iter().any(|ext| name.ends_with(ext))
}

/// The license and notice files at the top of `dir`, in name order.
pub fn files_in(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter(|entry| is_license_file(&entry.file_name().to_string_lossy()))
        .map(|entry| entry.path())
        .collect();
    files.sort();
    Ok(files)
}

/// Every file in `dir`, in name order, or none when `dir` does not exist.
pub fn all_files_in(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.path())
        .collect();
    files.sort();
    Ok(files)
}

/// The normalized, non-empty texts of `files`, without repeats.
pub fn read(files: &[PathBuf]) -> Result<Vec<String>, String> {
    let mut texts = Vec::new();
    for file in files {
        let raw = fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
        let text = normalize(&raw);
        if !text.is_empty() && !texts.contains(&text) {
            texts.push(text);
        }
    }
    Ok(texts)
}

/// Text with LF line ends, no byte order mark, no trailing spaces and no blank lines around it.
/// Bytes that are not UTF-8 are read as Latin-1.
pub fn normalize(raw: &[u8]) -> String {
    let text = String::from_utf8(raw.to_vec())
        .unwrap_or_else(|e| e.into_bytes().into_iter().map(char::from).collect());
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    let start = lines.iter().position(|line| !line.is_empty());
    let end = lines.iter().rposition(|line| !line.is_empty());
    match (start, end) {
        (Some(start), Some(end)) => lines[start..=end].join("\n"),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn license_files_are_recognised_by_name() {
        for name in [
            "LICENSE",
            "LICENSE-MIT",
            "license-apache-2.0",
            "LICENSE.md",
            "LICENCE",
            "UNLICENSE",
            "COPYING",
            "COPYRIGHT",
            "NOTICE.txt",
            "license",
        ] {
            assert!(is_license_file(name), "{name}");
        }
        for name in [
            "LICENSE.spdx",
            "README.md",
            "Cargo.toml",
            "license.rs",
            "notices.json",
        ] {
            assert!(!is_license_file(name), "{name}");
        }
    }

    #[test]
    fn normalize_drops_bom_carriage_returns_and_outer_blank_lines() {
        let raw = b"\xef\xbb\xbf\r\n\r\nMIT License  \r\n\r\nCopyright (c) Someone\t\r\n\r\n\r\n";
        assert_eq!(normalize(raw), "MIT License\n\nCopyright (c) Someone");
    }

    #[test]
    fn normalize_reads_latin1_when_the_bytes_are_not_utf8() {
        assert_eq!(
            normalize(b"Copyright \xa9 2020 Jos\xe9"),
            "Copyright © 2020 José"
        );
    }

    #[test]
    fn normalize_of_blank_text_is_empty() {
        assert_eq!(normalize(b" \r\n\n  \n"), "");
    }

    #[test]
    fn read_skips_repeated_and_empty_texts() {
        let dir = std::env::temp_dir().join(format!("vestigium-notices-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let files = ["LICENSE", "LICENSE.txt", "NOTICE"].map(|name| dir.join(name));
        fs::write(&files[0], "Same text\n").unwrap();
        fs::write(&files[1], "Same text\r\n").unwrap();
        fs::write(&files[2], "\n").unwrap();
        let texts = read(&files);
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(texts.unwrap(), vec!["Same text".to_owned()]);
    }
}
