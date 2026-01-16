Installer notes
- Inno Setup is installed at: C:\Program Files (x86)\Inno Setup 6
- Build with PowerShell: installer\build_installer.ps1
- powershell.exe -ExecutionPolicy Bypass -File "$(wslpath -w /home/hazzal/projects/HackmasterSim/installer/build_installer.ps1)"
- Set `WIN_TARGET` to package a non-default Windows target (e.g. `x86_64-pc-windows-gnu`), using binaries from `target\%WIN_TARGET%\release`.
- If `secrets\codesign\mygame-dev.cer` exists, the installer offers an optional task to trust the dev certificate for the current user.
