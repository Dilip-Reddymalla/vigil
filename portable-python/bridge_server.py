#!/usr/bin/env python3
"""Vigil python-dashboard bridge.

Serves the static dashboard UI (web/) on http://127.0.0.1:8765 and relays RPC
calls to the Vigil service over the named pipe, so the whole dashboard runs
inside a normal browser — no native Tauri exe required (Smart App Control safe,
since Python + the browser are trusted runtimes).

It also exposes updater endpoints so the browser dashboard can self-update by
downloading the latest `python-dashboard-*` release zip and applying it.

Stdlib-only: ctypes for the named pipe, http.server for the UI.
"""
import argparse
import ctypes
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request
import zipfile
from ctypes import wintypes as wt
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

GENERIC_READ = 0x80000000
GENERIC_WRITE = 0x40000000
OPEN_EXISTING = 3
INVALID_HANDLE_VALUE = -1
ERROR_PIPE_BUSY = 231

kernel32 = ctypes.windll.kernel32

# ---- Updater config ----
REPO = "Dilip-Reddymalla/vigil"
RELEASE_PREFIX = "python-dashboard-"
UA = {"User-Agent": "Vigil-Python-Dashboard"}
PKG_DIR = Path(__file__).resolve().parent
VERSION_FILE = PKG_DIR / "VERSION"


def installed_version():
    if VERSION_FILE.exists():
        v = VERSION_FILE.read_text(encoding="utf-8").strip()
        if v:
            return v
    return "0.1.5"  # fallback


def version_parts(v):
    return [int(x) for x in re.findall(r"\d+", v)][:4]


def is_newer(a, b):
    pa, pb = version_parts(a), version_parts(b)
    for x, y in zip(pa, pb):
        if x != y:
            return x > y
    return len(pa) > len(pb)


def gh_latest_python():
    """Return {version, url} for the newest python-dashboard release."""
    url = f"https://api.github.com/repos/{REPO}/releases"
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=20) as resp:
        releases = json.loads(resp.read().decode("utf-8"))
    best = None
    for rel in releases:
        tag = rel.get("tag_name") or ""
        if not tag.startswith(RELEASE_PREFIX):
            continue
        ver = tag[len(RELEASE_PREFIX):]
        if best is None or is_newer(ver, best[0]):
            assets = rel.get("assets") or []
            zip_asset = next((a for a in assets if a.get("name", "").endswith(".zip")), None)
            if zip_asset:
                dl = zip_asset.get("browser_download_url") or zip_asset.get("url")
                best = (ver, dl)
    if best is None:
        raise RuntimeError("no python-dashboard release found")
    return {"version": best[0], "url": best[1]}


def apply_update():
    """Download and apply the latest release; schedule service+bridge restart."""
    info = gh_latest_python()
    version = info["version"]

    tmp = Path(tempfile.mkdtemp(prefix="vigil-upd-"))
    zip_path = tmp / (version + ".zip")
    urllib.request.urlretrieve(info["url"], zip_path)

    with zipfile.ZipFile(zip_path) as z:
        for member in z.namelist():
            # sanitize / skip nested zips
            target = (PKG_DIR / member).resolve()
            if not str(target).startswith(str(PKG_DIR.resolve())):
                continue
            if member.endswith(".zip"):
                continue
            z.extract(member, PKG_DIR)

    VERSION_FILE.write_text(version, encoding="utf-8")
    shutil.rmtree(tmp, ignore_errors=True)

    schedule_restart()
    return version


def schedule_restart():
    """Detached helper: stop service, restart service + bridge after a short delay."""
    svc = PKG_DIR / "bin" / "vigil-service.exe"
    svc_str = str(svc).replace('"', '""')
    bridge = str(PKG_DIR / "bridge_server.py")
    cmd = PKG_DIR / "_restart.cmd"
    script = (
        "@echo off\n"
        "timeout /t 1 /nobreak >nul\n"
        f'taskkill /F /IM vigil-service.exe >nul 2>&1\n'
        f'start "" "{svc_str}"\n'
        "timeout /t 1 /nobreak >nul\n"
        f'where pythonw >nul 2>nul && start "" pythonw "{bridge}" --port 8765^'
        f' & goto :done\n'
        f'start "" python "{bridge}" --port 8765\n'
        ":done\n"
    )
    cmd.write_text(script, encoding="utf-8")
    subprocess.Popen(["cmd.exe", "/C", str(cmd)], creationflags=subprocess.CREATE_NO_WINDOW)


# ---- SID / named pipe ----
def current_sid():
    try:
        out = subprocess.check_output(["whoami", "/user"], text=True)
        m = re.search(r"S-1-\S+", out)
        if m:
            return m.group(0)
    except Exception:
        pass
    raise RuntimeError("cannot resolve user SID")


PIPE_NAME = r"\\.\pipe\vigil_" + current_sid()


def _open_pipe(name, tries=8):
    handle = kernel32.CreateFileW(
        name, GENERIC_READ | GENERIC_WRITE, 0, None, OPEN_EXISTING, 0, None,
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
            if len(out) > 1_000_000:
                break
        resp = json.loads(out.decode("utf-8", "replace").strip())
        if resp.get("error"):
            err = resp["error"]
            msg = err.get("message", "rpc error") if isinstance(err, dict) else str(err)
            raise RuntimeError(msg)
        return resp.get("result")
    finally:
        kernel32.CloseHandle(handle)


def send_json(handler, obj):
    payload = json.dumps(obj).encode("utf-8")
    handler.send_response(200)
    handler.send_header("Content-Type", "application/json")
    handler.send_header("Content-Length", str(len(payload)))
    handler.end_headers()
    handler.wfile.write(payload)


class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *a, **kw):
        web_dir = str(PKG_DIR / "web")
        super().__init__(*a, directory=web_dir, **kw)

    def log_message(self, fmt, *args):
        print("[http] " + (fmt % args), flush=True)

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        super().end_headers()

    # ---- updater endpoints ----
    def do_GET(self):
        if self.path == "/api/version":
            return send_json(self, {"installed": installed_version()})
        if self.path == "/api/update-check":
            try:
                info = gh_latest_python()
                inst = installed_version()
                send_json(self, {
                    "installed": inst,
                    "latest": info["version"],
                    "has_update": is_newer(info["version"], inst),
                })
            except Exception as exc:  # noqa: BLE001
                send_json(self, {"error": str(exc)})
            return
        if self.path == "/api/update-apply":
            return send_json(self, {"error": "use POST"})
        super().do_GET()

    def do_POST(self):
        if self.path == "/rpc":
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
            return
        if self.path == "/api/update-apply":
            try:
                version = apply_update()
                send_json(self, {"ok": True, "applied": version})
            except Exception as exc:  # noqa: BLE001
                send_json(self, {"ok": False, "error": str(exc)})
            return
        send_json(self, {"error": "unknown endpoint"})


def main():
    ap = argparse.ArgumentParser(description="Vigil python-dashboard bridge")
    ap.add_argument("--port", type=int, default=8765)
    ap.add_argument("--host", default="127.0.0.1")
    args = ap.parse_args()

    srv = ThreadingHTTPServer((args.host, args.port), Handler)
    print(f"Vigil bridge listening on http://{args.host}:{args.port}", flush=True)
    print(f"Pipe: {PIPE_NAME}", flush=True)
    print(f"Installed python-dashboard version: {installed_version()}", flush=True)
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        srv.shutdown()


if __name__ == "__main__":
    main()
