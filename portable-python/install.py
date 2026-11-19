#!/usr/bin/env python3
"""Vigil python-dashboard installer (replaces the NSIS setup on SAC systems).

Deploys the service + browser-dashboard package into
%LOCALAPPDATA%\Vigil, registers the service to auto-start, and writes the
dashboard-launcher.cmd the tray uses to open the browser dashboard.

Run from an elevated or normal shell:
    python install.py
"""
import os
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
LOCAL = Path(os.environ.get("LOCALAPPDATA", Path.home() / "AppData" / "Local"))
INSTALL_DIR = LOCAL / "Vigil"
PKG_DIR = INSTALL_DIR / "python-dashboard"
SERVICE_EXE = INSTALL_DIR / "vigil-service.exe"
LAUNCHER = INSTALL_DIR / "dashboard-launcher.cmd"
STARTUP_DIR = (
    Path(os.environ.get("APPDATA", ""))
    / "Microsoft" / "Windows" / "Start Menu" / "Programs" / "Startup"
)


def copy_package():
    print(f"[1/4] Deploying dashboard package -> {PKG_DIR}")
    if PKG_DIR.exists():
        shutil.rmtree(PKG_DIR)
    shutil.copytree(HERE, PKG_DIR, ignore=shutil.ignore_patterns("__pycache__"))


def ensure_service():
    # A bundled service (bin/vigil-service.exe) is optional; if present, install it.
    bundled = HERE / "bin" / "vigil-service.exe"
    if bundled.exists():
        print(f"[2/4] Installing service -> {SERVICE_EXE}")
        INSTALL_DIR.mkdir(parents=True, exist_ok=True)
        shutil.copy2(bundled, SERVICE_EXE)
    elif SERVICE_EXE.exists():
        print(f"[2/4] Service already installed at {SERVICE_EXE}")
    else:
        print("[2/4] WARN: no vigil-service.exe found. Install it into "
              f"{INSTALL_DIR} (or put one at portable-python/bin/).")
        raise SystemExit("missing service binary")


def write_launcher():
    launcher = """@echo off
setlocal
set "BASE=%LOCALAPPDATA%\\Vigil\\python-dashboard"
where pythonw >nul 2>nul && (start "" pythonw "%BASE%\\bridge_server.py" & goto :open)
where pyw >nul 2>nul && (start "" pyw "%BASE%\\bridge_server.py" & goto :open)
start "" python "%BASE%\\bridge_server.py"
:open
timeout /t 1 /nobreak >nul
start "" http://127.0.0.1:8765
"""
    LAUNCHER.write_text(launcher, encoding="utf-8")
    print(f"[3/4] Wrote tray launcher -> {LAUNCHER}")


def register_autostart():
    STARTUP_DIR.mkdir(parents=True, exist_ok=True)
    cmd = STARTUP_DIR / "Vigil Service.cmd"
    # start the service hidden (title ""), tolerate any path
    cmd.write_text(f'@echo off\nstart "" "{SERVICE_EXE}"\n', encoding="utf-8")
    print(f"[4/4] Registered service auto-start -> {cmd}")


def start_service_now():
    if SERVICE_EXE.exists():
        try:
            subprocess.Popen(
                [str(SERVICE_EXE)], creationflags=subprocess.CREATE_NO_WINDOW
            )
            print("Service started.")
        except Exception as exc:  # noqa: BLE001
            print(f"Could not start service: {exc}")


def main():
    print("== Vigil python-dashboard installer ==\n")
    if not sys.platform.startswith("win"):
        raise SystemExit("Windows only")
    copy_package()
    ensure_service()
    write_launcher()
    register_autostart()
    print("\nInstalled. Start the dashboard via the tray 'Open Dashboard' or run:")
    print(f'  "{LAUNCHER}"')
    ans = input("\nStart the bridge + open the dashboard now? [y/N] ").strip().lower()
    if ans == "y":
        subprocess.Popen([str(LAUNCHER)], creationflags=subprocess.CREATE_NO_WINDOW)
        start_service_now()


if __name__ == "__main__":
    main()
