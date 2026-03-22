@echo off
:: ═══════════════════════════════════════════════════════
::  CyberSnatcher Dev Launcher — .bat Entry Point
::  Launches the PowerShell WPF GUI application.
::
::  Place both this .bat and the .ps1 file in the same
::  directory, then double-click this .bat to start.
:: ═══════════════════════════════════════════════════════

title CyberSnatcher Launcher

:: Get this script's directory
set "SCRIPT_DIR=%~dp0"

:: Check that the PS1 file exists
if not exist "%SCRIPT_DIR%CyberSnatcher_Launcher.ps1" (
    echo.
    echo  [ERROR] CyberSnatcher_Launcher.ps1 not found!
    echo  Make sure it's in the same folder as this .bat file.
    echo.
    pause
    exit /b 1
)

:: Launch PowerShell with execution policy bypass, hidden console
:: -WindowStyle Hidden hides the PowerShell console window
:: -ExecutionPolicy Bypass allows the script to run without policy issues
powershell.exe -ExecutionPolicy Bypass -WindowStyle Hidden -File "%SCRIPT_DIR%CyberSnatcher_Launcher.ps1"

exit
