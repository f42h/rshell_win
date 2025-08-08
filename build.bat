@echo off

cargo build --release

if %errorlevel% neq 0 (
    echo Build failed!
    exit /b %errorlevel%
) else (
    echo Build complete!
    echo "Usage: target\release\rshell_win.exe <c2_address> <c2_port>"
)