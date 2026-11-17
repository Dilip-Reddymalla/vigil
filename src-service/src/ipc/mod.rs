//! Named-pipe JSON-RPC 2.0 IPC (the dashboard ↔ service channel).

pub mod rpc;
pub mod security;
pub mod server;

pub const PIPE_BUFFER_SIZE: u32 = 65536;
