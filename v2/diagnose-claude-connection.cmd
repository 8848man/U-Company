@echo off
rem Double-click entry: runs diagnose-claude-connection.ps1 without changing the system execution policy.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0diagnose-claude-connection.ps1" %*
pause
