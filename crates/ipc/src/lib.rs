//! Named pipe between the Vestigium agent and its interface.

use std::path::PathBuf;

mod frame;
mod pipe;
mod protocol;
mod security;

pub use frame::{MAX_FRAME, read_frame, write_frame};
pub use pipe::{PipeClient, PipeServer, pipe_name};
pub use protocol::{
    AgentStatus, ErrorCode, NowFrame, PROTOCOL_VERSION, Request, Response, TrayState,
};
pub use security::{current_user_sid, user_only_sddl};

/// Why a pipe operation failed.
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("pipe I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Windows error: {0}")]
    Win32(#[from] windows::core::Error),
    #[error("malformed message: {0}")]
    Json(#[from] serde_json::Error),
    #[error("message of {0} bytes is over the limit")]
    FrameTooLarge(u32),
    #[error("another agent already owns the pipe")]
    AlreadyRunning,
    #[error("the pipe belongs to an unexpected program: {}", found.display())]
    ServerMismatch { found: PathBuf },
}
