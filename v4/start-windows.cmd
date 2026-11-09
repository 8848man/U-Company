@echo off
rem Utopia Company v4 launcher: double-click to start. Close the app window to stop.
rem Optional: start-windows.cmd -DataDir .run-data\v4-test   (separate data)
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0start-windows.ps1" %*
if errorlevel 1 pause
