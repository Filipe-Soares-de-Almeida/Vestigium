use tray_icon::Icon;

const SIZE: u32 = 16;

/// A plain neutral square until the product has its own icon.
pub fn placeholder() -> Icon {
    let rgba = [0x80, 0x80, 0x80, 0xFF].repeat((SIZE * SIZE) as usize);
    Icon::from_rgba(rgba, SIZE, SIZE).expect("buffer matches the icon size")
}
