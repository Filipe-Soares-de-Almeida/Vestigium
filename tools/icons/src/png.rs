use vestigium_brand::Image;

/// An 8-bit RGBA PNG of the image, rows unfiltered.
pub fn encode(image: &Image) -> Vec<u8> {
    let stride = (image.size * 4) as usize;
    let mut raw = Vec::with_capacity((stride + 1) * image.size as usize);
    for row in image.rgba.chunks(stride) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&image.size.to_be_bytes());
    header.extend_from_slice(&image.size.to_be_bytes());
    // Bit depth 8, colour type RGBA, default compression, filtering and no interlace.
    header.extend_from_slice(&[8, 6, 0, 0, 0]);

    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut out, b"IHDR", &header);
    chunk(
        &mut out,
        b"IDAT",
        &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 10),
    );
    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// The CRC-32 a PNG chunk ends with (reflected, polynomial 0xEDB88320).
fn crc32(bytes: &[u8]) -> u32 {
    !bytes.iter().fold(!0u32, |crc, &byte| {
        (0..8).fold(crc ^ u32::from(byte), |crc, _| {
            if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crc_matches_the_reference_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn a_png_round_trips_through_inflate() {
        let image = Image {
            size: 2,
            rgba: (0..16).collect(),
        };
        let bytes = encode(&image);

        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&bytes[12..16], b"IHDR");
        let length = u32::from_be_bytes(bytes[33..37].try_into().unwrap()) as usize;
        assert_eq!(&bytes[37..41], b"IDAT");
        let raw = miniz_oxide::inflate::decompress_to_vec_zlib(&bytes[41..41 + length]).unwrap();
        assert_eq!(
            raw,
            [&[0][..], &image.rgba[..8], &[0], &image.rgba[8..]].concat()
        );
        assert!(bytes.ends_with(&[0xAE, 0x42, 0x60, 0x82]));
    }
}
