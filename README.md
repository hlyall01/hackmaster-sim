# Hackmaster Sim

## WSL build + Windows signing

### One-time: create a dev signing cert
```bash
export CODESIGN_PFX_PASSWORD="your-strong-password"
./scripts/create_cert.sh
```
This creates `secrets/codesign/mygame-dev.pfx` and `secrets/codesign/thumbprint.txt` on the Windows side.

### Build + sign from WSL
```bash
export CODESIGN_PFX_PASSWORD="your-strong-password"
./scripts/build_release_signed.sh
```
This builds in WSL and then signs any `target/**/release/*.exe` using Windows `signtool.exe`.

single command: HACKMASTER_SIGN_THUMBPRINT="$(tr -d '\r\n' < /home/hazzal/projects/HackmasterSim/secrets/codesign/thumbprint.txt)" powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(wslpath -w /home/hazzal/projects/HackmasterSim/installer/build_installer.ps1)"

### Notes
- If `signtool.exe` is missing, install the Windows SDK or Visual Studio Build Tools with the Windows SDK + Signing Tools components.
- Self-signed certs remove warnings only on your machine; SmartScreen will still warn on other machines.
