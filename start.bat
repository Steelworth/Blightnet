@echo off
setlocal EnableExtensions
cd /d "%~dp0"
set PATH=%USERPROFILE%\.cargo\bin;%PATH%

where cargo >nul 2>nul
if errorlevel 1 (
  if exist "target\release\blightnet.exe" goto run
  echo Rust/cargo is missing. Install it from https://rustup.rs then run start.bat again.
  pause
  exit /b 1
)

echo Building native Blightnet...
cargo build --release
if errorlevel 1 (
  echo Build failed.
  pause
  exit /b 1
)

:run
if exist "target\release\blightnet.exe" copy /Y "target\release\blightnet.exe" "blightnet.exe" >nul
echo Starting Blightnet...
"target\release\blightnet.exe" %*
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
