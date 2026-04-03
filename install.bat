@echo off
setlocal

set "INSTALL_DIR=%ProgramFiles%\Octo"

echo Building Octo (release)...
cargo build --release
if errorlevel 1 (
    echo Build failed.
    exit /b 1
)

echo Installing to %INSTALL_DIR%...
if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"
copy /y "target\release\octo.exe" "%INSTALL_DIR%\octo.exe"
copy /y "assets\octo.svg" "%INSTALL_DIR%\octo.svg"

:: Add to PATH via registry (current user)
echo Adding %INSTALL_DIR% to user PATH...
for /f "tokens=2*" %%A in ('reg query "HKCU\Environment" /v Path 2^>nul') do set "CURRENT_PATH=%%B"
echo %CURRENT_PATH% | findstr /i /c:"%INSTALL_DIR%" >nul
if errorlevel 1 (
    setx PATH "%CURRENT_PATH%;%INSTALL_DIR%"
)

:: Create Start Menu shortcut
set "SHORTCUT_DIR=%APPDATA%\Microsoft\Windows\Start Menu\Programs"
echo Creating Start Menu shortcut...
powershell -NoProfile -Command "$ws = New-Object -ComObject WScript.Shell; $sc = $ws.CreateShortcut('%SHORTCUT_DIR%\Octo.lnk'); $sc.TargetPath = '%INSTALL_DIR%\octo.exe'; $sc.WorkingDirectory = '%USERPROFILE%'; $sc.Description = 'Multi-format data viewer and editor'; $sc.Save()"

echo.
echo Octo installed successfully.
echo   Binary:   %INSTALL_DIR%\octo.exe
echo   Shortcut: %SHORTCUT_DIR%\Octo.lnk
echo.
echo You may need to restart your terminal for PATH changes to take effect.
endlocal
