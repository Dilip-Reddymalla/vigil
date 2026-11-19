#!/usr/bin/env python3
"""Vigil python-dashboard bridge.

Serves the static dashboard UI (web/) on http://127.0.0.1:8765 and relays RPC
calls to the Vigil service over the named pipe, so the whole dashboard runs
inside a normal browser — no native Tauri exe required (Smart App Control safe,
since Python + the browser are trusted runtimes).

Stdlib-only: ctypes for the named pipe, http.server for the UI.
"""
import argparse
import ctypes
import json
import os
import subprocess
import sys
import threading
import time
from ctypes import wintypes as wt
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

GENERIC_READ = 0x80000000
GENERIC_WRITE = 0x40000000
OPEN_EXISTING = 3
INVALID_HANDLE_VALUE = -1
ERROR_PIPE_BUSY = 231

kernel32 = ctypes.windll.kernel32


def current_sid():
    """Resolve the current user's SID (used in the pipe name)."""
    try:
        out = subprocess.check_output(["whoami", "/user"], text=True)
        for line in out.splitlines():
            line = line.strip()
            if line.startswith("S-1-"):
                return line
    except Exception:
        pass
    raise RuntimeError("cannot resolve user SID")


PIPE_NAME = r"\\.\pipe\vigil_" + current_sid()


def _open_pipe(name, tries=8):
    handle = kernel32.CreateFileW(
        name,
        GENERIC_READ | GENERIC_WRITE,
        0,
        None,
        OPEN_EXISTING,
        0,
        None,
    )
    if handle == INVALID_HANDLE_VALUE:
        err = kernel32.GetLastError()
        if err == ERROR_PIPE_BUSY and tries > 0:
            time.sleep(0.015)
            return _open_pipe(name, tries - 1)
        raise RuntimeError(f"connect to pipe failed (win32 error {err})")
    return handle


def rpc_call(method, params=None):
    handle = _open_pipe(PIPE_NAME)
    try:
        req = json.dumps(
            {"jsonrpc": "2.0", "method": method, "params": params or None, "id": 1}
        ) + "\n"
        payload = req.encode("utf-8")
        written = wt.DWORD()
        if not kernel32.WriteFile(handle, payload, len(payload), ctypes.byref(written), None):
            raise RuntimeError("pipe write failed")

        out = b""
        chunk = ctypes.create_string_buffer(4096)
        while b"\n" not in out:
            nread = wt.DWORD()
            ok = kernel32.ReadFile(handle, chunk, 4096, ctypes.byref(nread), None)
            if not ok:
                break
            out += chunk.raw[: nread.value]
            if len(out) > 1_000_000:  # safety
                break

        resp = json.loads(out.decode("utf-8", "replace").strip())
        if resp.get("error"):
            err = resp["error"]
            msg = err.get("message", "rpc error") if isinstance(err, dict) else str(err)
            raise RuntimeError(msg)
        return resp.get("result")
    finally:
        kernel32.CloseHandle(handle)


class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *a, **kw):
        # Serve files relative to the web/ dir next to this script.
        web_dir = str(Path(__file__).resolve().parent / "web")
        super().__init__(*a, directory=web_dir, **kw)

    def log_message(self, fmt, *args):  # quieter logs
        print("[http] " + (fmt % args), flush=True)

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        super().end_headers()

    def do_POST(self):
        if self.path != "/rpc":
            self.send_error(404)
            return
        try:
            length = int(self.headers.get("Content-Length", 0))
            body = json.loads(self.rfile.read(length).decode("utf-8") or "{}")
            method = str(body.get("method", ""))
            params = body.get("params")
            result = rpc_call(method, params)
            payload = json.dumps({"result": result}, default=str).encode("utf-8")
        except Exception as exc:  # noqa: BLE001
            payload = json.dumps({"error": str(exc)}).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)


def main():
    ap = argparse.ArgumentParser(description="Vigil python-dashboard bridge")
    ap.add_argument("--port", type=int, default=8765)
    ap.add_argument("--host", default="127.0.0.1")
    args = ap.parse_args()

    srv = ThreadingHTTPServer((args.host, args.port), Handler)
    print(f"Vigil bridge listening on http://{args.host}:{args.port}", flush=True)
    print(f"Pipe: {PIPE_NAME}", flush=True)
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        srv.shutdown()


if __name__ == "__main__":
    main()
