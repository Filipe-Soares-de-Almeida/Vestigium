//! Writes the icon files of Vestigium from the brand mark.
//!
//! ```text
//! vestigium-icons preview <dir>   the tray glyph in every state, taskbar and size, and a magnified sheet
//! ```

mod png;

use std::path::Path;
use std::process::ExitCode;
use std::{env, fs, io};

use vestigium_brand::{Badge, Image, Scheme, TrayInk};

const TRAY_STATES: [(&str, Badge); 5] = [
    ("normal", Badge::None),
    ("attention", Badge::Attention),
    ("problem", Badge::Problem),
    ("paused", Badge::Paused),
    ("sensitive", Badge::Sensitive),
];
/// The small icon at 100, 125, 150 and 200% display scale.
const TRAY_SIZES: [u32; 4] = [16, 20, 24, 32];
/// Taskbar backgrounds of Windows 11 in dark and light mode, for the sheet only.
const TASKBARS: [(&str, bool, [u8; 3]); 2] = [
    ("dark", true, [0x20, 0x20, 0x20]),
    ("light", false, [0xee, 0xf0, 0xf3]),
];

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.as_slice() {
        [mode, dir] if mode == "preview" => preview(Path::new(dir)),
        _ => {
            eprintln!("usage: vestigium-icons preview <dir>");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(written) => {
            println!("wrote {written} files");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("vestigium-icons: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Writes every tray state in the Red theme, and a sheet of them on both taskbars.
fn preview(dir: &Path) -> io::Result<usize> {
    fs::create_dir_all(dir)?;
    let mut written = 0;
    let mut rows = Vec::new();
    for (taskbar, dark, background) in TASKBARS {
        let colours = TrayInk::of(Scheme::Red, dark);
        for size in TRAY_SIZES {
            let mut row = Vec::new();
            for (state, badge) in TRAY_STATES {
                let image = vestigium_brand::tray(size, badge, colours);
                write(
                    &dir.join(format!("tray-{state}-{taskbar}-{size}.png")),
                    &image,
                )?;
                written += 1;
                row.push((image, background));
            }
            rows.push(row);
        }
    }
    write(&dir.join("tray-sheet.png"), &sheet(&rows, 6))?;
    Ok(written + 1)
}

fn write(path: &Path, image: &Image) -> io::Result<()> {
    fs::write(path, png::encode(image))
}

/// Lays `rows` of images out on their backgrounds, each pixel drawn `zoom` times larger.
fn sheet(rows: &[Vec<(Image, [u8; 3])>], zoom: u32) -> Image {
    let gap = 4;
    let largest = rows.iter().flatten().map(|(image, _)| image.size).max();
    let pitch = largest.unwrap_or(0) * zoom + gap * 2;
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0) as u32;
    let side = (columns * pitch).max(rows.len() as u32 * pitch);
    let mut rgba = vec![0u8; (side * side * 4) as usize];
    for (r, row) in rows.iter().enumerate() {
        for (c, (image, background)) in row.iter().enumerate() {
            let (left, top) = (c as u32 * pitch, r as u32 * pitch);
            for y in 0..pitch {
                for x in 0..pitch {
                    let (ix, iy) = (x.saturating_sub(gap) / zoom, y.saturating_sub(gap) / zoom);
                    let inside = x >= gap && y >= gap && ix < image.size && iy < image.size;
                    let [pr, pg, pb, pa] = if inside { image.pixel(ix, iy) } else { [0; 4] };
                    let a = u32::from(pa);
                    let mix = |fg: u8, bg: u8| {
                        ((u32::from(fg) * a + u32::from(bg) * (255 - a) + 127) / 255) as u8
                    };
                    let at = (((top + y) * side + left + x) * 4) as usize;
                    rgba[at..at + 4].copy_from_slice(&[
                        mix(pr, background[0]),
                        mix(pg, background[1]),
                        mix(pb, background[2]),
                        255,
                    ]);
                }
            }
        }
    }
    Image { size: side, rgba }
}
