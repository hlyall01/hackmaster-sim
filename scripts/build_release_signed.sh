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

echo "Building Windows release binaries (target: $WIN_TARGET)..."
(cd "$REPO_ROOT" && cargo build --release --target "$WIN_TARGET")

EXES=("$REPO_ROOT"/target/"$WIN_TARGET"/release/*.exe)
if [[ ! -e "${EXES[0]}" ]]; then
  for candidate in "$REPO_ROOT"/target/*/release/*.exe; do
    if [[ -e "$candidate" ]]; then
      candidate_dir="$(dirname "$candidate")"
      EXES=("$candidate_dir"/*.exe)
      break
    fi
  done
fi

if [[ ! -e "${EXES[0]}" ]]; then
  echo "No Windows .exe files found under target/**/release." >&2
  echo "Set a Windows target (e.g. x86_64-pc-windows-gnu) or copy outputs into target/release." >&2
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
