use std::io::{Read, Write};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::IpcError;

/// Largest message body accepted, in bytes.
pub const MAX_FRAME: u32 = 1 << 20;

/// Writes one message: its length as a little-endian u32, then its JSON.
pub fn write_frame<W: Write, T: Serialize>(w: &mut W, msg: &T) -> Result<(), IpcError> {
    let body = serde_json::to_vec(msg)?;
    let len = u32::try_from(body.len())
        .ok()
        .filter(|&len| len <= MAX_FRAME)
        .ok_or(IpcError::FrameTooLarge(u32::MAX))?;
    w.write_all(&len.to_le_bytes())?;
    w.write_all(&body)?;
    w.flush()?;
    Ok(())
}

/// Reads one message, refusing an announced length over `MAX_FRAME` before allocating.
pub fn read_frame<R: Read, T: DeserializeOwned>(r: &mut R) -> Result<T, IpcError> {
    let mut prefix = [0u8; 4];
    r.read_exact(&mut prefix)?;
    let len = u32::from_le_bytes(prefix);
    if len > MAX_FRAME {
        return Err(IpcError::FrameTooLarge(len));
    }
    let mut body = vec![0u8; len as usize];
    r.read_exact(&mut body)?;
    Ok(serde_json::from_slice(&body)?)
}
