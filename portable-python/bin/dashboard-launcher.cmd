@echo off
setlocal
rem Vigil python-dashboard launcher (placed beside the service so the tray finds it).
set "PKG=%LOCALAPPDATA%\Vigil\python-dashboard"
if exist "%PKG%\bridge_server.py" goto :have
set "PKG=%~dp0.."
:have
echo Starting Vigil bridge...
where pythonw >nul 2>nul && (start "" pythonw "%PKG%\bridge_server.py" --port 8765 & goto :open)
where pyw >nul 2>nul && (start "" pyw "%PKG%\bridge_server.py" --port 8765 & goto :open)
start "" python "%PKG%\bridge_server.py" --port 8765
:open
timeout /t 1 /nobreak >nul
start "" http://127.0.0.1:8765
