@echo off
setlocal

set "PROJECT_UNC=\\wsl$\Ubuntu\home\hazzal\projects\HackmasterSim"
set "VSDEVCMD=C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat"
set "TARGET_DIR=%LOCALAPPDATA%\HackmasterSim\target-msvc"
set "OUTPUT=%TARGET_DIR%\release\sim_gui.exe"
set "DESTINATION=%USERPROFILE%\Desktop\sim_gui.exe"

if not exist "%VSDEVCMD%" (
    echo Visual Studio build tools were not found:
    echo %VSDEVCMD%
    goto :failed
)

call "%VSDEVCMD%" -arch=x64 -host_arch=x64 >nul || goto :build_failed
pushd "%PROJECT_UNC%" || goto :build_failed

echo Building sim_gui.exe...
set "CARGO_INCREMENTAL=0"
set "CARGO_TARGET_DIR=%TARGET_DIR%"
cargo build --release --bin sim_gui
if errorlevel 1 (
    popd
    goto :build_failed
)
popd

if not exist "%OUTPUT%" (
    echo Build succeeded, but the executable was not found:
    echo %OUTPUT%
    goto :failed
)

copy /Y "%OUTPUT%" "%DESTINATION%" >nul || goto :copy_failed

echo Updated "%DESTINATION%".
endlocal
exit /b 0

:build_failed
echo Build failed.
goto :failed

:copy_failed
echo Copy failed. Close sim_gui.exe if it is currently running, then try again.

:failed
endlocal
exit /b 1
