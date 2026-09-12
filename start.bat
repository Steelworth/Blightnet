@echo off
cd /d "%~dp0"
if exist Blightnet.exe (
  Blightnet.exe
  exit /b %ERRORLEVEL%
)
if exist Hearthsong.exe (
  Hearthsong.exe
  exit /b %ERRORLEVEL%
)
where python >nul 2>nul
if %ERRORLEVEL%==0 (
  python serve.py
  exit /b %ERRORLEVEL%
)
where python3 >nul 2>nul
if %ERRORLEVEL%==0 (
  python3 serve.py
  exit /b %ERRORLEVEL%
)
echo Blightnet.exe is missing, and Python was not found.
echo Copy the whole blightnet folder, including Blightnet.exe.
pause
exit /b 1
