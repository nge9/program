@echo off
setlocal
cd /d "%~dp0"
if exist ".venv\Scripts\python.exe" goto run
where py >nul 2>nul
if errorlevel 1 goto usepython
py -3 -m venv .venv
if errorlevel 1 goto error
goto run
:usepython
where python >nul 2>nul
if errorlevel 1 goto missing
python -m venv .venv
if errorlevel 1 goto error
:run
".venv\Scripts\python.exe" -c "import PyQt6.QtWebEngineWidgets, PyQt6.QtWebChannel, zxingcpp, PIL" >nul 2>nul
if not errorlevel 1 goto launch
".venv\Scripts\python.exe" -m pip install "PyQt6>=6.8,<7" "PyQt6-WebEngine>=6.8,<7" "zxing-cpp>=2.2,<4" "Pillow>=10,<13"
if errorlevel 1 goto error
:launch
".venv\Scripts\python.exe" social_workspace.py
if errorlevel 1 goto error
exit /b 0
:missing
echo Install 64-bit Python 3.12 from python.org, then run this file again.
pause
exit /b 1
:error
echo.
echo Startup failed. Copy the error text above so it can be diagnosed.
pause
exit /b 1
