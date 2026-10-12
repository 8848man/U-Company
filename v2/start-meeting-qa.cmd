@echo off
rem Double-click entry for QA-38: runs start-meeting-qa.ps1 without changing the system execution policy.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0start-meeting-qa.ps1" %*
if errorlevel 1 pause
