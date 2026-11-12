use windows::Win32::Security::Cryptography::{BCRYPT_SHA256_ALG_HANDLE, BCryptHash};

/// The SHA-256 of `bytes` as 64 lower-case hexadecimal digits.
pub fn sha256_hex(bytes: &[u8]) -> Result<String, String> {
    let mut out = [0u8; 32];
    // SAFETY: the SHA-256 pseudo-handle needs no setup; input and output are live slices.
    let status = unsafe { BCryptHash(BCRYPT_SHA256_ALG_HANDLE, None, bytes, &mut out) };
    if status.is_err() {
        return Err(format!("BCryptHash failed: {:#x}", status.0));
    }
    Ok(out.iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_a_known_digest() {
        assert_eq!(
            sha256_hex(b"abc").unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
