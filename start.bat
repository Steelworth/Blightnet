@echo off
setlocal EnableExtensions
cd /d "%~dp0"
set PATH=%USERPROFILE%\.cargo\bin;%PATH%

where cargo >nul 2>nul
if errorlevel 1 (
  if exist "blightnet.exe" goto run
  if exist "target\release\blightnet.exe" goto copyrun
  echo This folder has no blightnet.exe and cargo is not installed.
  echo Download the Windows release from GitHub, or install Rust from https://rustup.rs
  pause
  exit /b 1
)

if exist "target\release\blightnet.exe" if not exist "src\main.rs" goto copyrun
if not exist "src\main.rs" if exist "blightnet.exe" goto run

echo Building native Blightnet...
cargo build --release
if errorlevel 1 (
  if exist "target\release\blightnet.exe" goto copyrun
  if exist "blightnet.exe" goto run
  echo Build failed. Install the Visual Studio C++ build tools, or use the GitHub release exe.
  pause
  exit /b 1
)

:copyrun
if exist "target\release\blightnet.exe" copy /Y "target\release\blightnet.exe" "blightnet.exe" >nul

:run
echo Starting Blightnet...
if exist "blightnet.exe" (
  "blightnet.exe" %*
) else (
  "target\release\blightnet.exe" %*
)
if errorlevel 1 (
  echo Blightnet exited with an error.
  if exist blightnet.log (
    echo Last lines of blightnet.log:
    powershell -NoProfile -Command "Get-Content -Tail 40 blightnet.log"
  )
  pause
  exit /b 1
)
exit /b 0
