@echo off
setlocal EnableExtensions
cd /d "%~dp0"

powershell -NoProfile -ExecutionPolicy Bypass -Command "try { Unblock-File -LiteralPath '%~dp0Blightnet.exe' -ErrorAction SilentlyContinue } catch {}" >nul 2>nul

if exist "Blightnet.exe.new" (
  if exist "Blightnet.exe" move /Y "Blightnet.exe" "Blightnet.exe.bak" >nul 2>nul
  move /Y "Blightnet.exe.new" "Blightnet.exe" >nul 2>nul
)

if exist "Blightnet.exe" (
  "Blightnet.exe"
  exit /b %ERRORLEVEL%
)
if exist "Hearthsong.exe" (
  "Hearthsong.exe"
  exit /b %ERRORLEVEL%
)

where py >nul 2>nul
if %ERRORLEVEL%==0 (
  py -3 "serve.py"
  exit /b %ERRORLEVEL%
)
where python >nul 2>nul
if %ERRORLEVEL%==0 (
  python "serve.py"
  exit /b %ERRORLEVEL%
)

echo Blightnet.exe is missing, and Python was not found.
echo Keep the whole unzipped folder together: Blightnet.exe next to index.html.
echo If you just downloaded from GitHub, do not open index.html from the folder.
pause
exit /b 1
