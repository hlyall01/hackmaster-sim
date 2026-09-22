#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

WIN_TARGET="${WIN_TARGET:-${CARGO_BUILD_TARGET:-}}"
if [[ -z "${WIN_TARGET}" ]]; then
  echo "WIN_TARGET (or CARGO_BUILD_TARGET) is required for Windows builds from WSL." >&2
  echo "Example: WIN_TARGET=x86_64-pc-windows-gnu" >&2
  exit 1
fi

echo "Building Windows simulator (target: $WIN_TARGET)..."
(cd "$REPO_ROOT" && cargo build --release --bin sim_gui --target "$WIN_TARGET")

# Name the current output explicitly: old app executables may remain in target/.
EXES=("$REPO_ROOT/target/$WIN_TARGET/release/sim_gui.exe")
if [[ ! -f "${EXES[0]}" ]]; then
  echo "Simulator executable not found: ${EXES[0]}" >&2
  exit 1
fi

WIN_SIGN_SCRIPT="$(wslpath -w "$SCRIPT_DIR/sign_exe.ps1")"

for exe in "${EXES[@]}"; do
  if [[ ! -f "$exe" ]]; then
    continue
  fi
  WIN_EXE="$(wslpath -w "$exe")"
  echo "Signing $(basename "$exe")..."
  if [[ -n "${CODESIGN_PFX_PASSWORD-}" ]]; then
    pw_escaped=${CODESIGN_PFX_PASSWORD//\'/\'\'}
    ps_cmd="\$env:CODESIGN_PFX_PASSWORD = '$pw_escaped'; & '$WIN_SIGN_SCRIPT' -File '$WIN_EXE'"
    powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "$ps_cmd"
  else
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$WIN_SIGN_SCRIPT" -File "$WIN_EXE"
  fi
done
