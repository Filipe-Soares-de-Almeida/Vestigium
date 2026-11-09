//! Sends requests to a running agent: `send <path-to-Vestigium.exe> <request JSON>...`,
//! for example `send target\release\Vestigium.exe '{"type":"set_sensitive","on":true}' '{"type":"import_captures"}'`.

use std::path::PathBuf;

use vestigium_ipc::{PROTOCOL_VERSION, PipeClient, Request, current_user_sid, pipe_name};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let agent: PathBuf = args
        .next()
        .ok_or("usage: send <path-to-Vestigium.exe> <request JSON>...")?
        .into();
    let agent = std::fs::canonicalize(&agent)?;
    let agent = PathBuf::from(agent.to_string_lossy().trim_start_matches(r"\\?\"));
    let mut client = PipeClient::connect(&pipe_name(&current_user_sid()?), &agent)?;
    client.call(&Request::Hello {
        protocol: PROTOCOL_VERSION,
    })?;
    for json in args {
        let request: Request = serde_json::from_str(&json)?;
        println!("{}", serde_json::to_string(&client.call(&request)?)?);
    }
    Ok(())
}
