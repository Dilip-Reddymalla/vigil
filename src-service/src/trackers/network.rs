//! Network tracker — interface-level byte counters via `GetIfTable2`.
//!
//! Polls every 30s, computes deltas, and detects counter resets (negative delta →
//! clamped to 0, baseline reset). Snapshot rows are written per interface.

use std::collections::HashMap;
use std::time::Duration;

use chrono::{Local, Utc};
use windows_sys::Win32::Foundation::WIN32_ERROR;
use windows_sys::Win32::NetworkManagement::IpHelper::{
    FreeMibTable, GetIfTable2, MIB_IF_TABLE2, IF_TYPE_ETHERNET_CSMACD, IF_TYPE_IEEE80211,
    IF_TYPE_SOFTWARE_LOOPBACK,
};

const POLL_SECONDS: u64 = 30;

struct IfState {
    sent: u64,
    recv: u64,
}

/// Spawn the network poller thread.
pub fn spawn() -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("vigil-network".to_string())
        .spawn(move || {
            let mut baseline: HashMap<u64, IfState> = HashMap::new();
            loop {
                std::thread::sleep(Duration::from_secs(POLL_SECONDS));
                poll(&mut baseline);
            }
        })
        .expect("spawn network tracker thread")
}

fn poll(baseline: &mut HashMap<u64, IfState>) {
    let Some(engine) = crate::STATE.get() else {
        return;
    };

    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    // SAFETY: GetIfTable2 allocates the table; we free it with FreeMibTable.
    let ret: WIN32_ERROR = unsafe { GetIfTable2(&mut table) };
    if ret != 0 || table.is_null() {
        crate::log::raw(&serde_json::json!({
            "event": "error", "op": "get_if_table", "error": ret
        }));
        return;
    }

    let entries = unsafe { (*table).NumEntries };
    let row0 = unsafe { (*table).Table.as_ptr() };
    let now = Utc::now();
    let local_date = Local::now().format("%Y-%m-%d").to_string();

    for i in 0..entries {
        let row = unsafe { row0.add(i as usize) };
        let intf_type = unsafe { (*row).Type };
        if intf_type == IF_TYPE_SOFTWARE_LOOPBACK {
            continue;
        }
        // Use the stable interface LUID (not the index, which can change across
        // reboots/reconnects) as the database identifier (H4).
        let luid = unsafe { (*row).InterfaceLuid.Value };
        let name = wide_to_string(unsafe { &(*row).Description });
        let intf_kind = classify(intf_type);

        let cur_sent = unsafe { (*row).OutOctets };
        let cur_recv = unsafe { (*row).InOctets };

        let (delta_sent, delta_recv) = match baseline.get(&luid) {
            Some(prev) => {
                let ds = if cur_sent < prev.sent { 0 } else { cur_sent - prev.sent };
                let dr = if cur_recv < prev.recv { 0 } else { cur_recv - prev.recv };
                (ds, dr)
            }
            None => (0, 0),
        };

        {
            let guard = engine.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = crate::db::network::insert_snapshot(
                guard.db_conn(),
                &name,
                &luid.to_string(),
                intf_kind,
                delta_sent,
                delta_recv,
                cur_sent,
                cur_recv,
                &now,
                &local_date,
            ) {
                crate::log::raw(&serde_json::json!({
                    "event": "error", "op": "network_insert", "error": e.to_string()
                }));
            }
        }

        baseline.insert(luid, IfState { sent: cur_sent, recv: cur_recv });
    }

    unsafe { FreeMibTable(table as *const core::ffi::c_void) };
}

fn classify(intf_type: u32) -> &'static str {
    match intf_type {
        IF_TYPE_IEEE80211 => "wifi",
        IF_TYPE_ETHERNET_CSMACD => "ethernet",
        _ => "other",
    }
}

fn wide_to_string(buf: &[u16; 257]) -> String {
    let len = buf.iter().position(|&u| u == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}
