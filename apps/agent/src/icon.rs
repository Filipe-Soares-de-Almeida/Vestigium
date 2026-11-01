use tray_icon::Icon;
use vestigium_brand::{Badge, Scheme, TrayInk};
use vestigium_core::ColorTheme;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SM_CXSMICON};
use windows::core::w;

const DEFAULT_DPI: u32 = 96;

/// How the taskbar draws the tray: dark or light, and the side of a small icon in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Taskbar {
    pub dark: bool,
    pub size: u32,
}

impl Taskbar {
    pub fn read() -> Taskbar {
        Taskbar {
            dark: dark_taskbar(),
            size: small_icon_size(),
        }
    }
}

/// The tray icon showing `badge`, in the taskbar's ink and the colour theme's accent.
pub fn tray(badge: Badge, theme: ColorTheme, taskbar: Taskbar) -> Icon {
    let colours = TrayInk::of(scheme(theme), taskbar.dark);
    let image = vestigium_brand::tray(taskbar.size, badge, colours);
    Icon::from_rgba(image.rgba, image.size, image.size).expect("the glyph is square RGBA")
}

fn scheme(theme: ColorTheme) -> Scheme {
    match theme {
        ColorTheme::Red => Scheme::Red,
        ColorTheme::Graphite => Scheme::Graphite,
        ColorTheme::Slate => Scheme::Slate,
        ColorTheme::Indigo => Scheme::Indigo,
    }
}

/// True unless Windows draws the taskbar light; without the setting it is dark.
fn dark_taskbar() -> bool {
    let (mut light, mut size) = (0u32, 4u32);
    // SAFETY: reads one DWORD into a buffer of the size passed; both strings are static.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut light).cast()),
            Some(&mut size),
        )
    };
    status != ERROR_SUCCESS || light == 0
}

/// The small icon side at the taskbar's DPI; at 96 DPI when the taskbar is not found.
fn small_icon_size() -> u32 {
    // SAFETY: looks a window up by its static class name.
    let dpi = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }
        // SAFETY: the handle was just found; a closed window yields 0.
        .map(|taskbar| unsafe { GetDpiForWindow(taskbar) })
        .unwrap_or(0);
    small_icon_size_at(if dpi == 0 { DEFAULT_DPI } else { dpi })
}

fn small_icon_size_at(dpi: u32) -> u32 {
    // SAFETY: a metric query with no pointers.
    let side = unsafe { GetSystemMetricsForDpi(SM_CXSMICON, dpi) };
    u32::try_from(side).map_or(16, |side| side.clamp(16, 64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_small_icon_grows_with_the_display_scale() {
        assert_eq!(small_icon_size_at(96), 16);
        assert_eq!(small_icon_size_at(120), 20);
        assert_eq!(small_icon_size_at(144), 24);
        assert_eq!(small_icon_size_at(192), 32);
    }

    #[test]
    fn every_state_and_theme_makes_an_icon() {
        let themes = [
            ColorTheme::Red,
            ColorTheme::Graphite,
            ColorTheme::Slate,
            ColorTheme::Indigo,
        ];
        let badges = [
            Badge::None,
            Badge::Attention,
            Badge::Problem,
            Badge::Paused,
            Badge::Sensitive,
        ];
        for theme in themes {
            for badge in badges {
                for dark in [true, false] {
                    tray(badge, theme, Taskbar { dark, size: 24 });
                }
            }
        }
    }

    #[test]
    fn the_taskbar_reads_without_failing() {
        let taskbar = Taskbar::read();
        assert!((16..=64).contains(&taskbar.size));
    }
}
