//! Asks a running agent for its status: `status <path-to-Vestigium.exe>`.

use std::path::PathBuf;

use vestigium_ipc::{PROTOCOL_VERSION, PipeClient, Request, current_user_sid, pipe_name};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent: PathBuf = std::env::args_os()
        .nth(1)
        .ok_or("usage: status <path-to-Vestigium.exe>")?
        .into();
    let agent = std::fs::canonicalize(&agent)?;
    let agent = PathBuf::from(agent.to_string_lossy().trim_start_matches(r"\\?\"));
    let mut client = PipeClient::connect(&pipe_name(&current_user_sid()?), &agent)?;
    println!(
        "{:?}",
        client.call(&Request::Hello {
            protocol: PROTOCOL_VERSION
        })?
    );
    println!("{:?}", client.call(&Request::Status)?);
    Ok(())
}
