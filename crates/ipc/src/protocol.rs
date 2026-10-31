use serde::{Deserialize, Serialize};

/// Version of the messages below; both ends refuse a different one.
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Hello { protocol: u32 },
    Status,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Hello {
        protocol: u32,
        agent_version: String,
    },
    Status(AgentStatus),
    Error {
        message: String,
    },
}

/// What the agent is doing right now.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentStatus {
    pub paused: bool,
    /// Unix epoch, milliseconds.
    pub started_at_ms: i64,
    pub last_sample_ms: Option<i64>,
    pub last_error: Option<String>,
}
