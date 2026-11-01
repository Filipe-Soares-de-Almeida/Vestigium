//! Writes the icon files of Vestigium from the brand mark.
//!
//! ```text
//! vestigium-icons app <dir>       icon.ico and the PNGs the app bundle names
//! vestigium-icons preview <dir>   the tray glyph and the app icon at every size, and magnified sheets
//! ```

mod ico;
mod png;

use std::path::Path;
use std::process::ExitCode;
use std::{env, fs, io};

use vestigium_brand::{Badge, Image, Scheme, TrayInk};

/// The sizes inside icon.ico: 16 from the 16 px drawing, the rest from the 48 grid.
const ICO_SIZES: [u32; 9] = [16, 20, 24, 32, 40, 48, 64, 96, 256];
/// The PNGs the bundle configuration names, and the 512 px source image.
const BUNDLE_PNGS: [(&str, u32); 4] = [
    ("32x32.png", 32),
    ("128x128.png", 128),
    ("128x128@2x.png", 256),
    ("icon.png", 512),
];
/// Window backgrounds in dark and light mode, for the app icon sheet only.
const BASES: [[u8; 3]; 2] = [[0x12, 0x12, 0x12], [0xeb, 0xeb, 0xeb]];
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
        [mode, dir] if mode == "app" => app(Path::new(dir)),
        [mode, dir] if mode == "preview" => preview(Path::new(dir)),
        _ => {
            eprintln!("usage: vestigium-icons app|preview <dir>");
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

/// Writes the app icon files: the ICO the executables embed and the bundle's PNGs.
fn app(dir: &Path) -> io::Result<usize> {
    fs::create_dir_all(dir)?;
    let images: Vec<Image> = ICO_SIZES
        .iter()
        .map(|&size| vestigium_brand::app_icon(size))
        .collect();
    fs::write(dir.join("icon.ico"), ico::encode(&images))?;
    for (name, size) in BUNDLE_PNGS {
        write(&dir.join(name), &vestigium_brand::app_icon(size))?;
    }
    Ok(1 + BUNDLE_PNGS.len())
}

/// Writes every tray state in the Red theme and every app icon size, with a sheet of each.
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

    let small: Vec<Image> = ICO_SIZES
        .into_iter()
        .filter(|&size| size <= 64)
        .map(vestigium_brand::app_icon)
        .collect();
    for image in &small {
        write(&dir.join(format!("app-{}.png", image.size)), image)?;
    }
    let rows: Vec<Vec<(Image, [u8; 3])>> = BASES
        .iter()
        .map(|&background| {
            small
                .iter()
                .map(|image| (image.clone(), background))
                .collect()
        })
        .collect();
    write(&dir.join("app-sheet.png"), &sheet(&rows, 4))?;
    Ok(written + small.len() + 2)
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
