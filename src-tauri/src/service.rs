//! Named-pipe JSON-RPC client used by the Tauri commands to talk to `vigil-service`.

#![cfg(feature = "tauri")]

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::io::FromRawHandle;

use serde_json::{json, Value};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, GENERIC_READ, GENERIC_WRITE, HANDLE,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, TOKEN_USER, TokenUser,
};
use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, OPEN_EXISTING, FILE_FLAGS_AND_ATTRIBUTES, FILE_ACCESS_RIGHTS, FILE_SHARE_MODE,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

const INVALID_HANDLE_VALUE: HANDLE = -1isize as _;
const TOKEN_QUERY: u32 = 0x0008;

/// ERROR_PIPE_BUSY — the service has no free pipe instance right now.
const ERROR_PIPE_BUSY: u32 = 231;
/// Retry the pipe connect a few times with a tiny delay to ride out bursts
/// (the dashboard fires several IPC calls concurrently).
const CONNECT_RETRIES: u32 = 8;

/// A minimal, synchronous JSON-RPC client over the Vigil named pipe.
pub struct ServiceClient;

impl ServiceClient {
    /// Call a method; returns the `result` value on success.
    pub fn call(method: &str, params: Option<Value>) -> Result<Value, String> {
        let pipe = pipe_name().ok_or("unable to resolve user SID")?;
        let wide: Vec<u16> = pipe.encode_utf16().chain(Some(0)).collect();

        // SAFETY: CreateFileW on the named pipe; retry on ERROR_PIPE_BUSY so
        // concurrent calls don't randomly fail. Handle wrapped in `File` on success.
        let handle = loop {
            let h = unsafe {
                CreateFileW(
                    wide.as_ptr(),
                    (GENERIC_READ | GENERIC_WRITE) as FILE_ACCESS_RIGHTS,
                    0 as FILE_SHARE_MODE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    0 as FILE_FLAGS_AND_ATTRIBUTES,
                    std::ptr::null_mut(),
                )
            };
            if h == INVALID_HANDLE_VALUE {
                let err = unsafe { GetLastError() };
                if err == ERROR_PIPE_BUSY && CONNECT_RETRIES > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(15));
                    continue;
                }
                return Err(format!("connect to pipe failed (win32 error {err})"));
            }
            break h;
        };

        let mut file = unsafe { File::from_raw_handle(handle as _) };

        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.unwrap_or(json!(null)),
            "id": 1,
        });
        let line = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        file.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
        file.write_all(b"\n").map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;

        let mut response = String::new();
        {
            let mut reader = BufReader::new(&file);
            reader.read_line(&mut response).map_err(|e| e.to_string())?;
        }

        let parsed: Value = serde_json::from_str(response.trim()).map_err(|e| e.to_string())?;
        if let Some(err) = parsed.get("error") {
            return Err(err.get("message").and_then(|m| m.as_str()).unwrap_or("rpc error").to_string());
        }
        parsed.get("result").cloned().ok_or_else(|| "missing result".into())
    }
}

/// `\\.\pipe\vigil_{USER_SID}`
fn pipe_name() -> Option<String> {
    let sid = current_user_sid()?;
    Some(format!(r"\\.\pipe\vigil_{sid}"))
}

fn current_user_sid() -> Option<String> {
    // SAFETY: standard token query + SID-to-string conversion.
    unsafe {
        let process = GetCurrentProcess();
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
            return None;
        }
        let mut len: u32 = 0;
        GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut len);
        let mut buf = vec![0u8; len as usize];
        let ok = GetTokenInformation(token, TokenUser, buf.as_mut_ptr() as *mut core::ffi::c_void, len, &mut len);
        CloseHandle(token);
        if ok == 0 {
            return None;
        }
        let user = buf.as_ptr() as *const TOKEN_USER;
        let sid: *mut core::ffi::c_void = (*user).User.Sid;

        let mut str_ptr: windows_sys::core::PWSTR = std::ptr::null_mut();
        if ConvertSidToStringSidW(sid, &mut str_ptr) == 0 {
            return None;
        }
        let mut n = 0usize;
        while *str_ptr.add(n) != 0 {
            n += 1;
        }
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(str_ptr, n));
        windows_sys::Win32::Foundation::LocalFree(str_ptr as _);
        Some(text)
    }
}
