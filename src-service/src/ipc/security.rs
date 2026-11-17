//! Named-pipe security: restrict access to the current user's SID via a DACL.

use std::ffi::c_void;
use std::mem::size_of;

use anyhow::Result;
use windows_sys::core::PWSTR;
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, LocalFree, GENERIC_READ, GENERIC_WRITE, HANDLE,
};
use windows_sys::Win32::Security::{
    AddAccessAllowedAce, GetTokenInformation, InitializeAcl, InitializeSecurityDescriptor,
    SetSecurityDescriptorDacl, ACL, SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR, TOKEN_USER,
    TokenUser,
};
use windows_sys::Win32::Security::Authorization::{ConvertSidToStringSidW, ConvertStringSidToSidW};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

const TOKEN_QUERY: u32 = 0x0008;
const ACL_REVISION: u32 = 2;
const SECURITY_DESCRIPTOR_REVISION: u32 = 1;
const ACL_BUFFER_SIZE: usize = 256;

/// Keep the ACL / security-descriptor buffers alive while the pipe is in use.
pub struct PipeSecurity {
    _acl: Vec<u8>,
    _sd: Vec<u8>,
    sa: SECURITY_ATTRIBUTES,
}

impl PipeSecurity {
    /// Build a security descriptor allowing the current user (full access).
    pub fn new() -> Result<Self> {
        let sid_string = current_user_sid_string()?;

        // Parse the SID string back into a binary SID (allocated by advapi32).
        let mut sid: PSid = std::ptr::null_mut();
        let wide = widestr(&sid_string);
        let ok = unsafe { ConvertStringSidToSidW(wide.as_ptr(), &mut sid) };
        if ok == 0 {
            anyhow::bail!(
                "ConvertStringSidToSidW failed for '{}': {}",
                sid_string,
                unsafe { GetLastError() }
            );
        }

        // Build the DACL.
        let mut acl = vec![0u8; ACL_BUFFER_SIZE];
        let acl_ptr = acl.as_mut_ptr() as *mut ACL;
        unsafe {
            if InitializeAcl(acl_ptr, ACL_BUFFER_SIZE as u32, ACL_REVISION) == 0 {
                LocalFree(sid as _);
                anyhow::bail!("InitializeAcl failed: {}", GetLastError());
            }
            if AddAccessAllowedAce(
                acl_ptr,
                ACL_REVISION,
                (GENERIC_READ | GENERIC_WRITE) as u32,
                sid,
            ) == 0
            {
                LocalFree(sid as _);
                anyhow::bail!("AddAccessAllowedAce failed: {}", GetLastError());
            }
            LocalFree(sid as _);
        }

        // Security descriptor.
        let mut sd = vec![0u8; size_of::<SECURITY_DESCRIPTOR>()];
        let sd_ptr = sd.as_mut_ptr();
        unsafe {
            if InitializeSecurityDescriptor(sd_ptr as *mut c_void, SECURITY_DESCRIPTOR_REVISION) == 0 {
                anyhow::bail!("InitializeSecurityDescriptor failed: {}", GetLastError());
            }
            if SetSecurityDescriptorDacl(sd_ptr as *mut c_void, 1, acl_ptr, 0) == 0 {
                anyhow::bail!("SetSecurityDescriptorDacl failed: {}", GetLastError());
            }
        }

        let sa = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd_ptr as *mut c_void,
            bInheritHandle: 0,
        };

        Ok(Self { _acl: acl, _sd: sd, sa })
    }

    /// Pointer to the `SECURITY_ATTRIBUTES` (valid while `self` is alive).
    pub fn as_ptr(&self) -> *const SECURITY_ATTRIBUTES {
        &self.sa
    }
}

type PSid = *mut c_void;

/// Current user's SID as a string (`S-1-5-…`).
pub fn current_user_sid_string() -> Result<String> {
    unsafe {
        let process = GetCurrentProcess();
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
            anyhow::bail!("OpenProcessToken failed: {}", GetLastError());
        }

        let mut len: u32 = 0;
        GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut len);
        let mut buf = vec![0u8; len as usize];
        let ok = GetTokenInformation(
            token,
            TokenUser,
            buf.as_mut_ptr() as *mut c_void,
            len,
            &mut len,
        );
        let last_err = GetLastError();
        CloseHandle(token);

        if ok == 0 {
            anyhow::bail!("GetTokenInformation failed: {last_err}");
        }

        let user = buf.as_ptr() as *const TOKEN_USER;
        let sid: PSid = (*user).User.Sid;

        let mut str_ptr: PWSTR = std::ptr::null_mut();
        if ConvertSidToStringSidW(sid, &mut str_ptr) == 0 {
            anyhow::bail!("ConvertSidToStringSidW failed: {}", GetLastError());
        }
        let text = read_pwstr(str_ptr);
        if !str_ptr.is_null() {
            LocalFree(str_ptr as _);
        }
        Ok(text)
    }
}

fn widestr(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn read_pwstr(ptr: PWSTR) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16_lossy(slice)
}
