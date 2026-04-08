@echo off
setlocal

:: Check for administrator privileges
net session >nul 2>&1
if errorlevel 1 (
    echo This installer requires administrator privileges.
    echo Right-click install.bat and select "Run as administrator".
    pause
    exit /b 1
)

set "INSTALL_DIR=%ProgramFiles%\Octa"
set "SCRIPT_DIR=%~dp0"

:: Check for pre-built binary first, then build from source
if exist "%SCRIPT_DIR%octa.exe" (
    echo Using pre-built binary.
    set "BINARY=%SCRIPT_DIR%octa.exe"
) else if exist "%SCRIPT_DIR%target\release\octa.exe" (
    echo Using previously built binary.
    set "BINARY=%SCRIPT_DIR%target\release\octa.exe"
) else (
    echo Building Octa (release)...
    cargo build --release
    if errorlevel 1 (
        echo Build failed. Install Rust from https://rustup.rs/ or download a pre-built release.
        exit /b 1
    )
    set "BINARY=%SCRIPT_DIR%target\release\octa.exe"
)

echo Installing to %INSTALL_DIR%...
if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"
copy /y "%BINARY%" "%INSTALL_DIR%\octa.exe"
copy /y "%SCRIPT_DIR%assets\octa.svg" "%INSTALL_DIR%\octa.svg"
copy /y "%SCRIPT_DIR%assets\octa.png" "%INSTALL_DIR%\octa.png"

:: Add to PATH via registry (current user)
echo Adding %INSTALL_DIR% to user PATH...
for /f "tokens=2*" %%A in ('reg query "HKCU\Environment" /v Path 2^>nul') do set "CURRENT_PATH=%%B"
echo %CURRENT_PATH% | findstr /i /c:"%INSTALL_DIR%" >nul
if errorlevel 1 (
    setx PATH "%CURRENT_PATH%;%INSTALL_DIR%"
)

:: Convert PNG to ICO if not already present and magick is available
if not exist "%INSTALL_DIR%\octa.ico" (
    if exist "%INSTALL_DIR%\octa.png" (
        where magick >nul 2>&1
        if not errorlevel 1 (
            echo Converting icon...
            magick "%INSTALL_DIR%\octa.png" -define icon:auto-resize=256,128,64,48,32,16 "%INSTALL_DIR%\octa.ico"
        )
    )
)

:: Create Start Menu shortcut
set "SHORTCUT_DIR=%APPDATA%\Microsoft\Windows\Start Menu\Programs"
echo Creating Start Menu shortcut...
if exist "%INSTALL_DIR%\octa.ico" (
    set "ICON_PATH=%INSTALL_DIR%\octa.ico"
) else (
    set "ICON_PATH=%INSTALL_DIR%\octa.exe"
)
powershell -NoProfile -Command ^
    "$ws = New-Object -ComObject WScript.Shell;" ^
    "$sc = $ws.CreateShortcut('%SHORTCUT_DIR%\Octa.lnk');" ^
    "$sc.TargetPath = '%INSTALL_DIR%\octa.exe';" ^
    "$sc.IconLocation = '%ICON_PATH%';" ^
    "$sc.WorkingDirectory = '%USERPROFILE%';" ^
    "$sc.Description = 'Multi-format data viewer and editor';" ^
    "$sc.Save()"

echo.
echo Octa installed successfully.
echo   Binary:   %INSTALL_DIR%\octa.exe
echo   Shortcut: %SHORTCUT_DIR%\Octa.lnk
echo.
echo You may need to restart your terminal for PATH changes to take effect.
endlocal
