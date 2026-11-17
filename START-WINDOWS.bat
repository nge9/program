@echo off
setlocal
cd /d "%~dp0"
where cargo >nul 2>nul
if errorlevel 1 goto missing
echo Building Social Workspace. The first build can take a few minutes.
cargo build --release --locked
if errorlevel 1 goto error
start "" "target\release\social-workspace.exe"
exit /b 0
:missing
echo Install Rust from https://rustup.rs and Visual Studio C++ Build Tools first.
pause
exit /b 1
:error
echo.
echo Build failed. Copy the first error above so it can be diagnosed.
pause
exit /b 1
