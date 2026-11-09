use vestigium_brand::Image;

use crate::png;

/// An icon file holding each image as a PNG, which Windows reads at every size.
pub fn encode(images: &[Image]) -> Vec<u8> {
    let pngs: Vec<Vec<u8>> = images.iter().map(png::encode).collect();
    let mut out = vec![0, 0, 1, 0];
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (image, png) in images.iter().zip(&pngs) {
        // A side of 256 is written as 0.
        let side = if image.size >= 256 {
            0
        } else {
            image.size as u8
        };
        out.extend_from_slice(&[side, side, 0, 0]);
        // One colour plane, 32 bits per pixel.
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += png.len();
    }
    for png in pngs {
        out.extend_from_slice(&png);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank(size: u32) -> Image {
        Image {
            size,
            rgba: vec![0; (size * size * 4) as usize],
        }
    }

    #[test]
    fn the_directory_points_at_each_png() {
        let bytes = encode(&[blank(16), blank(256)]);
        let entry = |i: usize| &bytes[6 + 16 * i..6 + 16 * (i + 1)];
        let word = |slice: &[u8]| u32::from_le_bytes(slice.try_into().unwrap()) as usize;

        assert_eq!(&bytes[..6], &[0, 0, 1, 0, 2, 0]);
        assert_eq!(entry(0)[0], 16);
        assert_eq!(entry(1)[0], 0);
        for i in 0..2 {
            let (length, offset) = (word(&entry(i)[8..12]), word(&entry(i)[12..16]));
            assert_eq!(&bytes[offset + 1..offset + 4], b"PNG");
            assert!(offset + length <= bytes.len());
        }
        assert_eq!(
            word(&entry(1)[12..16]),
            word(&entry(0)[12..16]) + word(&entry(0)[8..12])
        );
    }
}
