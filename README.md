# Hackmaster Sim

## Squad battler demo

Run the separate squad battler browser demo on port 8788:

```bash
cargo run --bin squad_battler_demo -- --port 8788
```

Then open `http://127.0.0.1:8788`.

QA commands:

```bash
cargo check --bin squad_battler_demo
cargo check --bin sim_gui
test ! -f src/bin/autobattler_v2_demo.rs || cargo check --bin autobattler_v2_demo
python3 scripts/squad_battler_api_smoke.py --base-url http://127.0.0.1:8788 --seed 8788
```

See [docs/squad_battler_qa.md](docs/squad_battler_qa.md) for the smoke script,
deterministic replay check, API shape checks, and integration checklist.

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

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(wslpath -w /home/hazzal/projects/HackmasterSim/installer/build_installer.ps1)"

### Notes
- If `signtool.exe` is missing, install the Windows SDK or Visual Studio Build Tools with the Windows SDK + Signing Tools components.
- Self-signed certs remove warnings only on your machine; SmartScreen will still warn on other machines.
