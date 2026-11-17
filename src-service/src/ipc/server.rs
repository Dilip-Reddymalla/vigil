//! Named-pipe server (JSON-RPC 2.0, newline-delimited, 64 KB frames).

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::io::FromRawHandle;

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_PIPE_CONNECTED};
use windows_sys::Win32::Storage::FileSystem::{FILE_FLAGS_AND_ATTRIBUTES, PIPE_ACCESS_DUPLEX};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, NAMED_PIPE_MODE, PIPE_ACCEPT_REMOTE_CLIENTS,
    PIPE_READMODE_MESSAGE, PIPE_TYPE_MESSAGE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};

use super::rpc::RpcError;
use super::security;

/// `\\.\pipe\vigil_{USER_SID}`
pub fn pipe_name(sid: &str) -> String {
    format!(r"\\.\pipe\vigil_{sid}")
}

fn widen(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// Serve the IPC endpoint forever (blocking). Intended to run on its own thread.
pub fn run() -> anyhow::Result<()> {
    let sid = security::current_user_sid_string()?;
    let sec = security::PipeSecurity::new()?;
    let name = widen(&pipe_name(&sid));

    loop {
        // SAFETY: create a fresh named-pipe instance with the user-restricted DACL.
        let hpipe = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX as FILE_FLAGS_AND_ATTRIBUTES,
                (PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT | PIPE_ACCEPT_REMOTE_CLIENTS)
                    as NAMED_PIPE_MODE,
                PIPE_UNLIMITED_INSTANCES,
                super::PIPE_BUFFER_SIZE,
                super::PIPE_BUFFER_SIZE,
                0,
                sec.as_ptr(),
            )
        };

        if hpipe.is_null() {
            crate::log::raw(&serde_json::json!({
                "event": "error", "op": "create_pipe", "error": unsafe { GetLastError() }
            }));
            continue;
        }

        let connected = unsafe { ConnectNamedPipe(hpipe, std::ptr::null_mut()) };
        if connected == 0 && unsafe { GetLastError() } != ERROR_PIPE_CONNECTED {
            // Client gone before connect completed; close and retry.
            unsafe { CloseHandle(hpipe) };
            continue;
        }

        // Wrap in a std File; ownership means it closes on drop.
        let mut file = unsafe { File::from_raw_handle(hpipe as _) };
        serve(&mut file);
    }
}

fn serve(file: &mut File) {
    loop {
        let mut line = String::new();
        let n = {
            let mut reader = BufReader::new(&*file);
            match reader.read_line(&mut line) {
                Ok(n) => n,
                Err(_) => break,
            }
        };
        if n == 0 {
            break; // client closed
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let response = handle_line(trimmed);
        if !response.is_empty() {
            if file.write_all(response.as_bytes()).is_err() || file.flush().is_err() {
                break;
            }
        }
    }
}

fn handle_line(line: &str) -> String {
    let req = match serde_json::from_str::<serde_json::Value>(line) {
        Ok(v) => v,
        Err(_) => return rpc_error(-32700, "Parse error", None),
    };

    let id = req.get("id").cloned();
    let method = match req.get("method").and_then(|m| m.as_str()) {
        Some(m) => m,
        None => return rpc_error(-32600, "Invalid Request", id),
    };

    let params = req.get("params");
    match dispatch_method(method, params) {
        Ok(result) => rpc_result(result, id),
        Err(e) => rpc_error(e.code(), &e.message(), id),
    }
}

fn dispatch_method(
    method: &str,
    params: Option<&serde_json::Value>,
) -> Result<serde_json::Value, RpcError> {
    let engine = crate::STATE.get().ok_or(RpcError::ServiceBusy)?;
    let mut guard = engine.lock().unwrap_or_else(|e| e.into_inner());
    super::rpc::dispatch(&mut guard, method, params)
}

fn rpc_result(result: serde_json::Value, id: Option<serde_json::Value>) -> String {
    let body = serde_json::json!({ "jsonrpc": "2.0", "result": result, "id": id });
    format!("{}\n", serde_json::to_string(&body).unwrap())
}

fn rpc_error(code: i64, message: &str, id: Option<serde_json::Value>) -> String {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "error": { "code": code, "message": message },
        "id": id
    });
    format!("{}\n", serde_json::to_string(&body).unwrap())
}
